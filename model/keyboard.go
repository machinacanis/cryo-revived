package model

// Keyboard is an inline keyboard. Use either a platform template ID or
// a custom layout built with KeyboardBuilder.
type Keyboard struct {
	ID      string           `json:"id,omitempty"`
	Content *KeyboardContent `json:"content,omitempty"`
}

type KeyboardContent struct {
	Rows []*Row `json:"rows"`
}

type Row struct {
	Buttons []*Button `json:"buttons"`
}

type Button struct {
	ID         string      `json:"id,omitempty"`
	GroupID    string      `json:"group_id,omitempty"` // same group greys out others after one click (action type 1)
	RenderData *RenderData `json:"render_data,omitempty"`
	Action     *Action     `json:"action,omitempty"`
}

type RenderData struct {
	Label        string `json:"label"` // ≤ 10 chars
	VisitedLabel string `json:"visited_label,omitempty"`
	Style        int    `json:"style"` // 0 grey 1 blue 3 white-red 4 blue-white
}

type Action struct {
	Type          int         `json:"type"` // 0=jump 1=callback 2=command
	Permission    *Permission `json:"permission,omitempty"`
	Data          string      `json:"data,omitempty"` // required for type 1/2
	UnsupportTips string      `json:"unsupport_tips,omitempty"`
	Enter         bool        `json:"enter,omitempty"`  // command button auto-send (c2c, qq>=8983)
	Reply         bool        `json:"reply,omitempty"`  // command with quote
	Anchor        int         `json:"anchor,omitempty"` // 1 = open image picker (overrides Enter)
	Modal         *Modal      `json:"modal,omitempty"`
}

type Permission struct {
	Type           int      `json:"type"` // 0=specified users 1=admin 2=everyone
	SpecifyUserIDs []string `json:"specify_user_ids,omitempty"`
	SpecifyRoleIDs []string `json:"specify_role_ids,omitempty"` // guild only
}

type Modal struct {
	Content     string `json:"content"`                // ≤40 chars, no URL
	ConfirmText string `json:"confirm_text,omitempty"` // ≤4 chars
	CancelText  string `json:"cancel_text,omitempty"`  // ≤4 chars
}

// --- builders ---

// KeyboardByID references a platform keyboard template.
func KeyboardByID(id string) *Keyboard { return &Keyboard{ID: id} }

// NewKeyboard starts a custom keyboard layout.
func NewKeyboard() *KeyboardBuilder { return &KeyboardBuilder{&Keyboard{Content: &KeyboardContent{}}} }

type KeyboardBuilder struct{ k *Keyboard }

// Row appends a row built from buttons.
func (b *KeyboardBuilder) Row(btns ...*Button) *KeyboardBuilder {
	b.k.Content.Rows = append(b.k.Content.Rows, &Row{Buttons: btns})
	return b
}

// Build returns the keyboard for Msg().Keyboard(...).
func (b *KeyboardBuilder) Build() *Keyboard { return b.k }

// NewButton creates a button with the given visible label.
func NewButton(label string) *Button {
	return &Button{RenderData: &RenderData{Label: label}}
}

// WithID sets the unique button id inside one keyboard.
func (b *Button) WithID(id string) *Button { b.ID = id; return b }
func (b *Button) Style(s int) *Button      { b.RenderData.Style = s; return b }
func (b *Button) VisitedLabel(s string) *Button {
	b.RenderData.VisitedLabel = s
	return b
}
func (b *Button) Group(id string) *Button { b.GroupID = id; return b }

// Jump makes the button open a URL or miniapp (type 0).
func (b *Button) Jump(data string) *Button { return b.act(0, data) }

// Callback posts button data back to the bot backend (type 1).
func (b *Button) Callback(data string) *Button { return b.act(1, data) }

// Command inserts "@bot data" into the input box (type 2).
func (b *Button) Command(data string) *Button { return b.act(2, data) }

func (b *Button) act(t int, data string) *Button {
	if b.Action == nil {
		b.Action = &Action{}
	}
	b.Action.Type, b.Action.Data = t, data
	return b
}

// Perm restricts who may click: 0=specified users 1=admin 2=everyone.
func (b *Button) Perm(t int, userIDs ...string) *Button {
	if b.Action == nil {
		b.Action = &Action{}
	}
	b.Action.Permission = &Permission{Type: t, SpecifyUserIDs: userIDs}
	return b
}

func (b *Button) Enter(v bool) *Button {
	if b.Action != nil {
		b.Action.Enter = v
	}
	return b
}
func (b *Button) QuoteReply(v bool) *Button {
	if b.Action != nil {
		b.Action.Reply = v
	}
	return b
}

// Confirm adds a modal second-confirmation prompt.
func (b *Button) Confirm(content, confirm, cancel string) *Button {
	if b.Action == nil {
		b.Action = &Action{}
	}
	b.Action.Modal = &Modal{Content: content, ConfirmText: confirm, CancelText: cancel}
	return b
}
