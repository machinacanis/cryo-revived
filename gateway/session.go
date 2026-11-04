// Package gateway maintains the websocket event session.
package gateway

import (
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"log/slog"
	"sync"
	"time"

	"github.com/gorilla/websocket"

	"cryo/api"
	"cryo/model"
)

// DispatchFunc is invoked for every incoming payload.
type DispatchFunc func(*model.Payload)

// Session is one sharded gateway connection.
type Session struct {
	API     *api.Client
	Intents model.Intent
	Shard   [2]int
	Logger  *slog.Logger

	OnEvent   DispatchFunc                             // every payload (including READY/RESUMED)
	OnSession func(sessionID string, user *model.User) // after READY

	conn      *websocket.Conn
	mu        sync.Mutex
	sessionID string
	seq       int
	lastHB    time.Time
}

// Run connects and serves events until ctx is done or a fatal error occurs.
// It automatically resumes the session on recoverable disconnects.
func (s *Session) Run(ctx context.Context) error {
	s.log().Info("gateway: resolving entry")
	url, err := s.API.GetWSS(ctx)
	if err != nil {
		return fmt.Errorf("gateway: get wss url: %w", err)
	}
	for {
		err := s.serve(ctx, url)
		if ctx.Err() != nil {
			return ctx.Err()
		}
		if errors.Is(err, errNoResume) {
			s.sessionID, s.seq = "", 0
			s.log().Warn("gateway: session invalidated, re-identify", "err", err)
			continue
		}
		if err != nil {
			s.log().Warn("gateway: reconnecting", "err", err)
			select {
			case <-ctx.Done():
				return ctx.Err()
			case <-time.After(3 * time.Second):
			}
			continue
		}
		return nil
	}
}

var errNoResume = errors.New("session cannot be resumed")

// serve runs one connection lifecycle: hello → auth → read loop.
func (s *Session) serve(ctx context.Context, url string) error {
	conn, _, err := websocket.DefaultDialer.DialContext(ctx, url, nil)
	if err != nil {
		return fmt.Errorf("dial: %w", err)
	}
	s.mu.Lock()
	s.conn = conn
	s.mu.Unlock()
	defer conn.Close()

	// Hello
	var hello model.Payload
	if err := conn.ReadJSON(&hello); err != nil {
		return fmt.Errorf("read hello: %w", err)
	}
	if hello.Op != model.OpHello {
		return fmt.Errorf("expect hello(op10), got op=%d", hello.Op)
	}
	var hb struct {
		Interval int `json:"heartbeat_interval"`
	}
	_ = json.Unmarshal(hello.D, &hb)
	interval := time.Duration(hb.Interval) * time.Millisecond
	if interval <= 0 {
		interval = 45 * time.Second
	}

	// Identify or Resume
	if s.sessionID == "" {
		if err := s.identify(); err != nil {
			return err
		}
	} else {
		if err := s.resume(); err != nil {
			return err
		}
	}

	errc := make(chan error, 1)
	go s.heartbeatLoop(ctx, interval, errc)
	go s.readLoop(errc)

	select {
	case <-ctx.Done():
		return ctx.Err()
	case err := <-errc:
		return err
	}
}

func (s *Session) identify() error {
	tok, err := s.API.Token(context.Background())
	if err != nil {
		return err
	}
	return s.send(&model.Payload{Op: model.OpIdentify, D: mustJSON(map[string]interface{}{
		"token":   "QQBot " + tok,
		"intents": int(s.Intents),
		"shard":   s.Shard,
		"properties": map[string]string{
			"$os": "windows", "$browser": "cryo", "$device": "cryo",
		},
	})})
}

func (s *Session) resume() error {
	tok, err := s.API.Token(context.Background())
	if err != nil {
		return err
	}
	return s.send(&model.Payload{Op: model.OpResume, D: mustJSON(map[string]interface{}{
		"token": "QQBot " + tok, "session_id": s.sessionID, "seq": s.seq,
	})})
}

func (s *Session) send(p *model.Payload) error {
	s.mu.Lock()
	defer s.mu.Unlock()
	return s.conn.WriteJSON(p)
}

func (s *Session) heartbeatLoop(ctx context.Context, interval time.Duration, errc chan<- error) {
	t := time.NewTicker(interval)
	defer t.Stop()
	for {
		select {
		case <-ctx.Done():
			return
		case <-t.C:
			s.mu.Lock()
			seq := s.seq
			s.mu.Unlock()
			if err := s.send(&model.Payload{Op: model.OpHeartbeat, D: mustJSON(seqOrNull(seq))}); err != nil {
				tryErr(errc, fmt.Errorf("heartbeat: %w", err))
				return
			}
		}
	}
}

func seqOrNull(seq int) interface{} {
	if seq == 0 {
		return nil
	}
	return seq
}

func (s *Session) readLoop(errc chan<- error) {
	for {
		var p model.Payload
		if err := s.conn.ReadJSON(&p); err != nil {
			if ce, ok := err.(*websocket.CloseError); ok {
				tryErr(errc, classifyClose(ce.Code, ce.Text))
			} else {
				tryErr(errc, fmt.Errorf("read: %w", err))
			}
			return
		}
		s.handle(&p)
	}
}

func (s *Session) handle(p *model.Payload) {
	if p.S > 0 {
		s.mu.Lock()
		s.seq = p.S
		s.mu.Unlock()
	}
	switch p.Op {
	case model.OpDispatch:
		if p.T == model.EventReady {
			var r struct {
				SessionID string      `json:"session_id"`
				User      *model.User `json:"user"`
			}
			_ = json.Unmarshal(p.D, &r)
			s.sessionID = r.SessionID
			if s.OnSession != nil {
				s.OnSession(r.SessionID, r.User)
			}
		}
	case model.OpReconnect:
		// server will close; readLoop will surface the close error
	case model.OpInvalidSession:
		s.log().Warn("gateway: invalid session")
	}
	if s.OnEvent != nil {
		s.OnEvent(p)
	}
}

// classifyClose maps gateway close codes to resumable/fatal errors.
func classifyClose(code int, text string) error {
	switch {
	case code == 4009, code == 4008:
		return fmt.Errorf("close %d (resumable): %s", code, text)
	case code >= 4900 && code <= 4913, code == 4006, code == 4007:
		return fmt.Errorf("%w: close %d: %s", errNoResume, code, text)
	case code == 4914, code == 4915:
		return fmt.Errorf("fatal close %d (banned/offline): %s", code, text)
	default:
		return fmt.Errorf("close %d: %s", code, text)
	}
}

func mustJSON(v interface{}) json.RawMessage {
	b, err := json.Marshal(v)
	if err != nil {
		panic(err)
	}
	return b
}

func tryErr(errc chan<- error, err error) {
	select {
	case errc <- err:
	default:
	}
}

func (s *Session) log() *slog.Logger {
	if s.Logger != nil {
		return s.Logger
	}
	return slog.Default()
}
