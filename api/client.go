// Package api implements the QQ Bot OpenAPI REST client.
package api

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"sync"
	"time"
)

const baseURL = "https://api.bot.qq.com"

// Client calls the OpenAPI. Obtain one via cryo.Bot.API().
type Client struct {
	appID     string
	secret    string
	http      *http.Client
	tokenMu   sync.Mutex
	token     string
	tokenExpr time.Time
}

// NewClient builds a client for the given credentials.
func NewClient(appID, appSecret string) *Client {
	return &Client{appID: appID, secret: appSecret,
		http: &http.Client{Timeout: 30 * time.Second}}
}

// APIError is a business failure returned by the platform (err_code != 0).
type APIError struct {
	Code    int    `json:"err_code"`
	Message string `json:"message"`
	TraceID string `json:"trace_id"`
	Status  int    `json:"-"`
}

func (e *APIError) Error() string {
	return fmt.Sprintf("qqbot api: err_code=%d status=%d msg=%s trace=%s",
		e.Code, e.Status, e.Message, e.TraceID)
}

// Token returns a valid access_token, refreshing when needed.
// Tokens are cached; the platform returns the same token until it nears expiry.
func (c *Client) Token(ctx context.Context) (string, error) {
	c.tokenMu.Lock()
	defer c.tokenMu.Unlock()
	if c.token != "" && time.Until(c.tokenExpr) > time.Minute {
		return c.token, nil
	}
	return c.refreshLocked(ctx)
}

func (c *Client) refreshLocked(ctx context.Context) (string, error) {
	var out struct {
		AccessToken string      `json:"access_token"`
		ExpiresIn   json.Number `json:"expires_in"`
		Code        int         `json:"code"`
		Message     string      `json:"message"`
	}
	err := c.raw(ctx, http.MethodPost, baseURL+"/app/getAppAccessToken", nil,
		map[string]string{"appId": c.appID, "clientSecret": c.secret}, &out)
	if err != nil {
		return "", err
	}
	if out.Code != 0 {
		return "", &APIError{Code: out.Code, Message: out.Message}
	}
	if out.AccessToken == "" {
		return "", &APIError{Message: "empty access_token"}
	}
	secs, _ := out.ExpiresIn.Int64()
	if secs <= 0 {
		secs = 7200
	}
	c.token = out.AccessToken
	c.tokenExpr = time.Now().Add(time.Duration(secs) * time.Second)
	return c.token, nil
}

// do performs an authenticated request and decodes the JSON body into out.
// Business errors surface as *APIError even when the HTTP status is 200.
func (c *Client) do(ctx context.Context, method, path string, body, out interface{}) error {
	tok, err := c.Token(ctx)
	if err != nil {
		return err
	}
	return c.raw(ctx, method, baseURL+path, map[string]string{
		"Authorization": "QQBot " + tok,
	}, body, out)
}

func (c *Client) raw(ctx context.Context, method, url string, headers map[string]string, body, out interface{}) error {
	var rdr io.Reader
	if body != nil {
		buf, err := json.Marshal(body)
		if err != nil {
			return err
		}
		rdr = bytes.NewReader(buf)
	}
	req, err := http.NewRequestWithContext(ctx, method, url, rdr)
	if err != nil {
		return err
	}
	req.Header.Set("Content-Type", "application/json; charset=utf-8")
	for k, v := range headers {
		req.Header.Set(k, v)
	}
	resp, err := c.http.Do(req)
	if err != nil {
		return err
	}
	defer resp.Body.Close()
	data, err := io.ReadAll(resp.Body)
	if err != nil {
		return err
	}
	if out != nil && len(data) > 0 {
		if err := json.Unmarshal(data, out); err != nil {
			return fmt.Errorf("qqbot api: decode %s %s: %w", method, url, err)
		}
	}
	// business error check: envelope uses err_code (or code for token api)
	var probe struct {
		ErrCode int    `json:"err_code"`
		Code    int    `json:"code"`
		Message string `json:"message"`
		TraceID string `json:"trace_id"`
	}
	_ = json.Unmarshal(data, &probe)
	if probe.ErrCode != 0 || probe.Code != 0 {
		return &APIError{Code: probe.ErrCode + probe.Code, Message: probe.Message,
			TraceID: orStr(probe.TraceID, resp.Header.Get("X-Tps-trace-ID")), Status: resp.StatusCode}
	}
	if resp.StatusCode >= 400 {
		return &APIError{Code: -resp.StatusCode, Message: string(data),
			TraceID: resp.Header.Get("X-Tps-trace-ID"), Status: resp.StatusCode}
	}
	return nil
}

func orStr(a, b string) string {
	if a != "" {
		return a
	}
	return b
}
