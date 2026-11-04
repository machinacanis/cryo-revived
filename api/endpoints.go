package api

import (
	"context"
	"fmt"

	"cryo/model"
)

// --- identity-bound endpoint handles ---
//
// Each method on Client returns a small handle bound to one openid/id, so
// calls read naturally: api.User(openid).SendMessage(ctx, msg).

// User returns the endpoint handle of one c2c user.
func (c *Client) User(openid string) *UserAPI { return &UserAPI{c, "/v2/users/" + openid} }

// Group returns the endpoint handle of one QQ group.
func (c *Client) Group(openid string) *GroupAPI { return &GroupAPI{c, "/v2/groups/" + openid} }

// Channel returns the endpoint handle of one guild sub-channel.
func (c *Client) Channel(id string) *ChannelAPI { return &ChannelAPI{c, id} }

// Guild returns the endpoint handle of one guild.
func (c *Client) Guild(id string) *GuildAPI { return &GuildAPI{c, id} }

// --- bot self ---

// Me returns the bot's own profile.
func (c *Client) Me(ctx context.Context) (*model.User, error) {
	var u model.User
	return &u, c.do(ctx, "GET", "/users/@me", nil, &u)
}

// GetGuilds lists guilds the bot joined.
func (c *Client) GetGuilds(ctx context.Context) ([]*model.Guild, error) {
	var gs []*model.Guild
	return gs, c.do(ctx, "GET", "/users/@me/guilds", nil, &gs)
}

// GenerateURLLink creates a share link carrying callback_data.
func (c *Client) GenerateURLLink(ctx context.Context, callbackData string) (string, error) {
	var out struct {
		URL string `json:"url"`
	}
	err := c.do(ctx, "POST", "/v2/generate_url_link",
		map[string]string{"callback_data": callbackData}, &out)
	return out.URL, err
}

// --- gateway ---

// GetWSS returns the generic websocket entry point.
func (c *Client) GetWSS(ctx context.Context) (string, error) {
	var out struct {
		URL string `json:"url"`
	}
	return out.URL, c.do(ctx, "GET", "/gateway", nil, &out)
}

// WSSShard describes a sharded websocket entry and session limits.
type WSSShard struct {
	URL               string `json:"url"`
	Shards            int    `json:"shards"`
	SessionStartLimit struct {
		Total          int `json:"total"`
		Remaining      int `json:"remaining"`
		ResetAfter     int `json:"reset_after"`
		MaxConcurrency int `json:"max_concurrency"`
	} `json:"session_start_limit"`
}

// GetWSSShard returns the recommended shard count and entry point.
func (c *Client) GetWSSShard(ctx context.Context) (*WSSShard, error) {
	var s WSSShard
	return &s, c.do(ctx, "GET", "/gateway/bot", nil, &s)
}

// --- c2c user endpoints ---

type UserAPI struct {
	c    *Client
	base string
}

// SendMessage delivers a message to this user.
func (u *UserAPI) SendMessage(ctx context.Context, m *model.MessageToCreate) (*model.PostMessageResult, error) {
	var r model.PostMessageResult
	return &r, u.c.do(ctx, "POST", u.base+"/messages", m, &r)
}

// SendText is a shortcut for SendMessage(ctx, model.NewMessage().Text(s)).
func (u *UserAPI) SendText(ctx context.Context, s string) (*model.PostMessageResult, error) {
	return u.SendMessage(ctx, model.NewMessage().Text(s))
}

// RecallMessage withdraws a message sent within 2 minutes.
func (u *UserAPI) RecallMessage(ctx context.Context, msgID string) error {
	return u.c.do(ctx, "DELETE", u.base+"/messages/"+msgID, nil, nil)
}

// SendStreamMessage sends one chunk of a streaming (typewriter) markdown message.
func (u *UserAPI) SendStreamMessage(ctx context.Context, s *model.StreamMessage) (*model.StreamMessageResult, error) {
	var r model.StreamMessageResult
	return &r, u.c.do(ctx, "POST", u.base+"/stream_messages", s, &r)
}

// UploadFileURL uploads media by URL and returns file_info for Msg().FileInfo.
func (u *UserAPI) UploadFileURL(ctx context.Context, fileType int, url string) (string, error) {
	f, err := u.upload(ctx, &fileReq{FileType: fileType, URL: url})
	if err != nil {
		return "", err
	}
	return f.FileInfo, nil
}

// UploadFileData uploads raw bytes (≤5MB) and returns file_info.
func (u *UserAPI) UploadFileData(ctx context.Context, fileType int, name string, data []byte) (string, error) {
	f, err := u.upload(ctx, &fileReq{FileType: fileType, FileName: name, FileData: string(data)})
	if err != nil {
		return "", err
	}
	return f.FileInfo, nil
}

func (u *UserAPI) upload(ctx context.Context, req *fileReq) (*fileResp, error) {
	var r fileResp
	return &r, u.c.do(ctx, "POST", u.base+"/files", req, &r)
}

// --- group endpoints ---

type GroupAPI struct {
	c    *Client
	base string
}

// SendMessage delivers a message to this group.
func (g *GroupAPI) SendMessage(ctx context.Context, m *model.MessageToCreate) (*model.PostMessageResult, error) {
	var r model.PostMessageResult
	return &r, g.c.do(ctx, "POST", g.base+"/messages", m, &r)
}

func (g *GroupAPI) SendText(ctx context.Context, s string) (*model.PostMessageResult, error) {
	return g.SendMessage(ctx, model.NewMessage().Text(s))
}

// RecallMessage withdraws a group message.
func (g *GroupAPI) RecallMessage(ctx context.Context, msgID string) error {
	return g.c.do(ctx, "DELETE", g.base+"/messages/"+msgID, nil, nil)
}

// UploadFileURL uploads media by URL and returns file_info (not shared with c2c).
func (g *GroupAPI) UploadFileURL(ctx context.Context, fileType int, url string) (string, error) {
	var r fileResp
	err := g.c.do(ctx, "POST", g.base+"/files",
		&fileReq{FileType: fileType, URL: url}, &r)
	return r.FileInfo, err
}

// GetInfo returns group profile.
func (g *GroupAPI) GetInfo(ctx context.Context) (*GroupInfo, error) {
	var i GroupInfo
	return &i, g.c.do(ctx, "GET", g.base+"/info", nil, &i)
}

// GetMembers returns one page of members; empty cursor starts from the head.
func (g *GroupAPI) GetMembers(ctx context.Context, cursor string) (*MemberPage, error) {
	var p MemberPage
	err := g.c.do(ctx, "GET", g.base+"/members", map[string]string{"cursor": cursor}, &p)
	return &p, err
}

// GetMember returns one member's profile.
func (g *GroupAPI) GetMember(ctx context.Context, memberOpenID string) (*GroupMember, error) {
	var m GroupMember
	err := g.c.do(ctx, "GET", g.base+"/members/"+memberOpenID, nil, &m)
	return &m, err
}

// RemoveMembers removes up to 20 members; blacklist=true also bans them.
func (g *GroupAPI) RemoveMembers(ctx context.Context, blacklist bool, memberOpenIDs ...string) error {
	return g.c.do(ctx, "POST", g.base+"/batch_remove_members", map[string]interface{}{
		"member_openids": memberOpenIDs, "add_to_member_blacklist": blacklist,
	}, nil)
}

// MuteMembers sets member mute states; op: "add"|"update"|"del".
func (g *GroupAPI) MuteMembers(ctx context.Context, op string, expireAt string, memberOpenIDs ...string) error {
	members := make([]map[string]string, 0, len(memberOpenIDs))
	for _, id := range memberOpenIDs {
		members = append(members, map[string]string{
			"op": op, "member_openid": id, "mute_expire_at": expireAt,
		})
	}
	return g.c.do(ctx, "POST", g.base+"/restrict_chat_setting",
		map[string]interface{}{"members": members}, nil)
}

// --- channel / guild endpoints ---

type ChannelAPI struct {
	c  *Client
	id string
}

// SendMessage posts to a guild channel (legacy payload: content/embed/ark/markdown/image).
func (ch *ChannelAPI) SendMessage(ctx context.Context, m *model.ChannelMessageToCreate) (string, error) {
	var out struct {
		ID string `json:"id"`
	}
	err := ch.c.do(ctx, "POST", fmt.Sprintf("/channels/%s/messages", ch.id), m, &out)
	return out.ID, err
}

func (ch *ChannelAPI) RecallMessage(ctx context.Context, msgID string) error {
	return ch.c.do(ctx, "DELETE",
		fmt.Sprintf("/channels/%s/messages/%s?hidetip=false", ch.id, msgID), nil, nil)
}

// GetInfo returns the channel object.
func (ch *ChannelAPI) GetInfo(ctx context.Context) (*model.Channel, error) {
	var c model.Channel
	return &c, ch.c.do(ctx, "GET", "/channels/"+ch.id, nil, &c)
}

type GuildAPI struct {
	c  *Client
	id string
}

func (g *GuildAPI) GetInfo(ctx context.Context) (*model.Guild, error) {
	var gr model.Guild
	return &gr, g.c.do(ctx, "GET", "/guilds/"+g.id, nil, &gr)
}

func (g *GuildAPI) GetChannels(ctx context.Context) ([]*model.Channel, error) {
	var cs []*model.Channel
	return cs, g.c.do(ctx, "GET", "/guilds/"+g.id+"/channels", nil, &cs)
}

func (g *GuildAPI) GetMembers(ctx context.Context, limit int, after string) ([]*model.Member, error) {
	var ms []*model.Member
	err := g.c.do(ctx, "GET",
		fmt.Sprintf("/guilds/%s/members?limit=%d&after=%s", g.id, limit, after), nil, &ms)
	return ms, err
}

// --- interactions ---

// RespondInteraction acks an INTERACTION_CREATE event (required for type 11/12).
// code: 0 ok, 1 failed, 2 frequent, 3 repeated, 4 no perm, 5 admin only.
func (c *Client) RespondInteraction(ctx context.Context, interactionID string, code int) error {
	return c.do(ctx, "PUT", "/interactions/"+interactionID,
		map[string]int{"code": code}, nil)
}

// --- shared request/response types ---

type fileReq struct {
	FileType   int    `json:"file_type"` // 1 image 2 video 3 voice 4 file
	URL        string `json:"url,omitempty"`
	SrvSendMsg bool   `json:"srv_send_msg,omitempty"`
	UploadID   string `json:"upload_id,omitempty"`
	FileName   string `json:"file_name,omitempty"`
	FileData   string `json:"file_data,omitempty"`
}

type fileResp struct {
	FileInfo string `json:"file_info"`
	TTL      int    `json:"ttl"`
	ID       string `json:"id"`
}

// GroupInfo of a QQ group.
type GroupInfo struct {
	GroupOpenID     string   `json:"group_openid"`
	GroupName       string   `json:"group_name"`
	GroupFingerMemo string   `json:"group_finger_memo"`
	GroupClassText  string   `json:"group_class_text"`
	GroupTags       []string `json:"group_tags"`
	GroupMemberNum  int      `json:"group_member_num"`
}

// GroupMember of a QQ group.
type GroupMember struct {
	MemberOpenID string `json:"member_openid"`
	Nick         string `json:"nick,omitempty"`
	JoinTime     string `json:"join_timestamp,omitempty"`
	Role         string `json:"member_role,omitempty"`
}

// MemberPage is one page of the group member list.
type MemberPage struct {
	Members    []*GroupMember `json:"members"`
	NextCursor string         `json:"next_cursor"`
}
