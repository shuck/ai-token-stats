package main

import (
	"bufio"
	"bytes"
	"database/sql"
	"encoding/json"
	"io"
	"os"
	"path/filepath"
	"regexp"
	"sort"
	"strconv"
	"strings"
	"time"

	"github.com/klauspost/compress/zstd"
	_ "modernc.org/sqlite"
)

const (
	shanghaiZone  = "Asia/Shanghai"
	agentAll      = "all"
	agentCodex    = "Codex"
	agentZcode    = "ZCode"
	agentClaude   = "Claude"
	agentOpenCode = "OpenCode"
	agentDeepSeek = "DeepSeek"
)

var shanghai *time.Location

func init() {
	shanghai = time.FixedZone("CST", 8*3600)
}

type usage struct {
	Input     int64
	Cached    int64
	CacheWrite int64
	Output    int64
	Reasoning int64
	Total     int64
}

type record struct {
	ThreadID      string
	Agent         string
	Model         string
	Key           string
	Path          string
	Ts            int64
	Date          string
	Usage         usage
	ContextWindow *int64
}

type daySummary struct {
	Date             string
	Input            int64
	Cached           int64
	Output           int64
	Reasoning        int64
	Total            int64
	Turns            int
	MaxContextWindow *int64
	MaxUsagePercent  *float64
	HitRate          *float64
	ByModel          map[string]*daySummary
	ByAgent          map[string]*daySummary
}

type report struct {
	GeneratedAt string
	Timezone    string
	Days        int
	RangeStart  string
	RangeEnd    string
	Totals      daySummary
	Today       daySummary
	Daily       []daySummary
	Models      []string
	Agents      []string
}

type rawEvent struct {
	Type      string          `json:"type"`
	Timestamp string          `json:"timestamp"`
	Payload   json.RawMessage `json:"payload"`
}

type sessionMetaPayload struct {
	SessionID string `json:"session_id"`
}

type taskStartedPayload struct {
	ModelContextWindow *int64 `json:"model_context_window"`
}

type tokenUsageJSON struct {
	Input     int64 `json:"input_tokens"`
	Cached    int64 `json:"cached_input_tokens"`
	CacheWrite int64 `json:"cache_write_input_tokens"`
	Output    int64 `json:"output_tokens"`
	Reasoning int64 `json:"reasoning_output_tokens"`
	Total     int64 `json:"total_tokens"`
}

type tokenCountInfo struct {
	LastTokenUsage     *tokenUsageJSON `json:"last_token_usage"`
	TotalTokenUsage    *tokenUsageJSON `json:"total_token_usage"`
	ModelContextWindow *int64          `json:"model_context_window"`
}

type tokenCountPayload struct {
	Info tokenCountInfo `json:"info"`
}

type zcodeTokenJSON struct {
	Total     int64 `json:"total"`
	Input     int64 `json:"input"`
	Output    int64 `json:"output"`
	Reasoning int64 `json:"reasoning"`
	Cache     struct {
		Read  int64 `json:"read"`
		Write int64 `json:"write"`
	} `json:"cache"`
}

type zcodeMessageJSON struct {
	ModelID string          `json:"modelID"`
	Model   zcodeModelJSON  `json:"model"`
	Tokens  zcodeTokenJSON  `json:"tokens"`
}

type zcodeModelJSON struct {
	ModelID string `json:"modelID"`
}

func dateKey(ms int64) string {
	return time.UnixMilli(ms).In(shanghai).Format("2006-01-02")
}

func todayKey() string {
	return time.Now().In(shanghai).Format("2006-01-02")
}

func parseTimestamp(value string) int64 {
	if value == "" {
		return 0
	}
	t, err := time.Parse(time.RFC3339Nano, value)
	if err != nil {
		return 0
	}
	return t.UnixMilli()
}

func walkJSONL(root string, visit func(path string)) {
	filepath.Walk(root, func(path string, info os.FileInfo, err error) error {
		if err != nil {
			return nil
		}
		if !info.IsDir() && strings.HasSuffix(strings.ToLower(info.Name()), ".jsonl") {
			visit(path)
		}
		return nil
	})
}

func parseUsage(u *tokenUsageJSON) usage {
	if u == nil {
		return usage{}
	}
	total := u.Total
	if total == 0 {
		total = u.Input + u.Output
	}
	return usage{
		Input:      u.Input,
		Cached:     u.Cached,
		CacheWrite: u.CacheWrite,
		Output:     u.Output,
		Reasoning:  u.Reasoning,
		Total:      total,
	}
}

func loadRolloutRecords(changed map[string]bool) ([]record, map[string]bool, map[string]int64) {
	records := []record{}
	withTokenCount := map[string]bool{}
	contextByThread := map[string]int64{}

	visit := func(path string) {
		if changed != nil && !changed[path] {
			return
		}
		file, err := os.Open(path)
		if err != nil {
			return
		}
		defer file.Close()

		threadID := ""
		scanner := bufio.NewScanner(file)
		scanner.Buffer(make([]byte, 1024*1024), 16*1024*1024)
		for scanner.Scan() {
			var event rawEvent
			if err := json.Unmarshal(scanner.Bytes(), &event); err != nil {
				continue
			}
			if event.Type == "session_meta" {
				var meta sessionMetaPayload
				if json.Unmarshal(event.Payload, &meta) == nil && meta.SessionID != "" {
					threadID = meta.SessionID
				}
				continue
			}
			if event.Type != "event_msg" {
				continue
			}
			var msg struct {
				Type string `json:"type"`
			}
			if json.Unmarshal(event.Payload, &msg) != nil {
				continue
			}
			switch msg.Type {
			case "task_started":
				var payload taskStartedPayload
				if json.Unmarshal(event.Payload, &payload) == nil && payload.ModelContextWindow != nil && threadID != "" {
					contextByThread[threadID] = *payload.ModelContextWindow
				}
			case "token_count":
				var payload tokenCountPayload
				if json.Unmarshal(event.Payload, &payload) != nil || threadID == "" {
					continue
				}
				u := parseUsage(payload.Info.LastTokenUsage)
				if payload.Info.LastTokenUsage == nil {
					u = parseUsage(payload.Info.TotalTokenUsage)
				}
				ts := parseTimestamp(event.Timestamp)
				if ts == 0 {
					continue
				}
				withTokenCount[threadID] = true
				var ctx *int64
				if payload.Info.ModelContextWindow != nil {
					ctx = payload.Info.ModelContextWindow
				} else if value, ok := contextByThread[threadID]; ok {
					v := value
					ctx = &v
				}
				records = append(records, record{
					ThreadID:      threadID,
					Agent:         agentCodex,
					Key:           threadID + ":" + strconv.FormatInt(ts, 10) + ":" + strconv.FormatInt(u.Input, 10) + ":" + strconv.FormatInt(u.Output, 10),
					Path:          path,
					Ts:            ts,
					Date:          dateKey(ts),
					Usage:         u,
					ContextWindow: ctx,
				})
			}
		}
	}

	if changed == nil {
		walkJSONL(sessionsRoot(), visit)
		walkJSONL(archivedRoot(), visit)
	} else {
		for path := range changed {
			if strings.HasSuffix(strings.ToLower(path), ".jsonl") {
				visit(path)
			}
		}
	}
	threadModels := loadThreadModels()
	logModels := loadLogModels()
	for i := range records {
		if model, ok := threadModels[records[i].ThreadID]; ok {
			records[i].Model = model
		} else if model, ok := logModels[records[i].ThreadID]; ok {
			records[i].Model = model
		} else {
			records[i].Model = "unknown"
		}
	}
	return records, withTokenCount, contextByThread
}

var tokenUsageRe = regexp.MustCompile(`codex\.turn\.token_usage\.([a-z_]+)=(\d+)`)
var turnIDRe = regexp.MustCompile(`turn\.id=([A-Za-z0-9_-]+)`)
var turnIDRe2 = regexp.MustCompile(`turn_id=([A-Za-z0-9_-]+)`)
var contextLimitRe = regexp.MustCompile(`full_context_window_limit=Some\((\d+)\)`)
var modelRe = regexp.MustCompile(`model=([A-Za-z0-9_.:-]+)`)

func loadThreadModels() map[string]string {
	result := map[string]string{}
	if _, err := os.Stat(stateDB()); err != nil {
		return result
	}
	db, err := sql.Open("sqlite", "file:"+stateDB()+"?mode=ro&_pragma=query_only(1)")
	if err != nil {
		return result
	}
	defer db.Close()
	rows, err := db.Query(`SELECT id, model FROM threads WHERE model IS NOT NULL AND model != ''`)
	if err != nil {
		return result
	}
	defer rows.Close()
	for rows.Next() {
		var id, model string
		if rows.Scan(&id, &model) == nil {
			result[id] = model
		}
	}
	return result
}

func loadLogModels() map[string]string {
	result := map[string]string{}
	if _, err := os.Stat(logsDB()); err != nil {
		return result
	}
	db, err := sql.Open("sqlite", "file:"+logsDB()+"?mode=ro&_pragma=query_only(1)")
	if err != nil {
		return result
	}
	defer db.Close()
	rows, err := db.Query(
		`SELECT thread_id, feedback_log_body FROM logs
		  WHERE thread_id IS NOT NULL AND feedback_log_body LIKE '%model=%'`)
	if err != nil {
		return result
	}
	defer rows.Close()
	for rows.Next() {
		var threadID, body string
		if rows.Scan(&threadID, &body) != nil || threadID == "" {
			continue
		}
		if _, ok := result[threadID]; ok {
			continue
		}
		if m := modelRe.FindStringSubmatch(body); len(m) == 2 {
			result[threadID] = m[1]
		}
	}
	return result
}

func loadLogFallback(withTokenCount map[string]bool, contextByThread map[string]int64, since int64) ([]record, int64) {
	if _, err := os.Stat(logsDB()); err != nil {
		return nil, 0
	}
	db, err := sql.Open("sqlite", "file:"+logsDB()+"?mode=ro&_pragma=query_only(1)")
	if err != nil {
		return nil, 0
	}
	defer db.Close()

	rows, err := db.Query(
		`SELECT ts, thread_id, feedback_log_body FROM logs
		  WHERE feedback_log_body LIKE '%codex.turn.token_usage.input_tokens%' AND ts > ?
		  ORDER BY ts`)
	if err != nil {
		return nil, 0
	}
	defer rows.Close()

	records := []record{}
	maxTs := since
	logModels := loadLogModels()
	seen := map[string]bool{}
	contextCache := map[string]*int64{}

	for rows.Next() {
		var ts int64
		var threadID string
		var body string
		if err := rows.Scan(&ts, &threadID, &body); err != nil || threadID == "" {
			continue
		}
		if ts > maxTs {
			maxTs = ts
		}
		if withTokenCount[threadID] {
			continue
		}
		turnID := ""
		if m := turnIDRe.FindStringSubmatch(body); len(m) == 2 {
			turnID = m[1]
		} else if m := turnIDRe2.FindStringSubmatch(body); len(m) == 2 {
			turnID = m[1]
		} else {
			turnID = strconv.FormatInt(ts, 10)
		}
		key := threadID + ":" + turnID
		if seen[key] {
			continue
		}
		seen[key] = true

		u := usage{}
		found := false
		model := "unknown"
		if m := modelRe.FindStringSubmatch(body); len(m) == 2 {
			model = m[1]
		} else if value, ok := logModels[threadID]; ok {
			model = value
		}
		for _, m := range tokenUsageRe.FindAllStringSubmatch(body, -1) {
			value, _ := strconv.ParseInt(m[2], 10, 64)
			switch m[1] {
			case "input_tokens":
				u.Input = value
				found = true
			case "cached_input_tokens":
				u.Cached = value
			case "cache_write_input_tokens":
				u.CacheWrite = value
			case "output_tokens":
				u.Output = value
			case "reasoning_output_tokens":
				u.Reasoning = value
			case "total_tokens":
				u.Total = value
			}
		}
		if !found {
			continue
		}
		if u.Total == 0 {
			u.Total = u.Input + u.Output
		}

		ctx := contextByThread[threadID]
		var ctxPtr *int64
		if ctx != 0 {
			v := ctx
			ctxPtr = &v
		} else if cached, ok := contextCache[threadID]; ok {
			ctxPtr = cached
		} else {
			var contextRow string
			err := db.QueryRow(
				`SELECT feedback_log_body FROM logs
				  WHERE thread_id = ? AND feedback_log_body LIKE '%full_context_window_limit=Some(%'
				  ORDER BY ts DESC LIMIT 1`, threadID).Scan(&contextRow)
			if err == nil {
				if m := contextLimitRe.FindStringSubmatch(contextRow); len(m) == 2 {
					if value, err := strconv.ParseInt(m[1], 10, 64); err == nil {
						ctxPtr = &value
					}
				}
			}
			contextCache[threadID] = ctxPtr
		}

		ms := ts * 1000
		records = append(records, record{
			ThreadID:      threadID,
			Agent:         agentCodex,
			Key:           threadID + ":" + turnID,
			Path:          "logs",
			Model:         model,
			Ts:            ms,
			Date:          dateKey(ms),
			Usage:         u,
			ContextWindow: ctxPtr,
		})
	}
	return records, maxTs
}

func loadZCodeRecords(path string, since int64) ([]record, int64) {
	if _, err := os.Stat(path); err != nil {
		return nil, 0
	}
	db, err := sql.Open("sqlite", "file:"+path+"?mode=ro&_pragma=query_only(1)")
	if err != nil {
		return nil, 0
	}
	defer db.Close()
	rows, err := db.Query(
		`SELECT id, time_created, time_updated, data FROM message
		  WHERE json_extract(data, '$.tokens') IS NOT NULL AND time_updated > ?`, since)
	if err != nil {
		return nil, 0
	}
	defer rows.Close()

	records := []record{}
	maxUpdated := since
	for rows.Next() {
		var id string
		var created, updated int64
		var raw string
		if err := rows.Scan(&id, &created, &updated, &raw); err != nil {
			continue
		}
		if updated > maxUpdated {
			maxUpdated = updated
		}
		var msg zcodeMessageJSON
		if json.Unmarshal([]byte(raw), &msg) != nil {
			continue
		}
		model := msg.ModelID
		if model == "" {
			model = msg.Model.ModelID
		}
		if model == "" {
			model = "unknown"
		}
		records = append(records, record{
			ThreadID: "zcode",
			Agent:    agentZcode,
			Model:    model,
			Key:      id,
			Path:     "zcode",
			Ts:       created,
			Date:     dateKey(created),
			Usage: usage{
				Input:      msg.Tokens.Input,
				Cached:     msg.Tokens.Cache.Read,
				CacheWrite: msg.Tokens.Cache.Write,
				Output:     msg.Tokens.Output,
				Reasoning:  msg.Tokens.Reasoning,
				Total:      msg.Tokens.Total,
			},
			ContextWindow: nil,
		})
	}
	return records, maxUpdated
}

func loadCodexRecords(changed map[string]bool, logsSince int64) ([]record, int64) {
	records, withTokenCount, contextByThread := loadRolloutRecords(changed)
	logs, maxLogsTs := loadLogFallback(withTokenCount, contextByThread, logsSince)
	return append(records, logs...), maxLogsTs
}

type claudeMessageJSON struct {
	Model string `json:"model"`
	Usage struct {
		InputTokens               int64 `json:"input_tokens"`
		CacheReadInputTokens      int64 `json:"cache_read_input_tokens"`
		CacheCreationInputTokens  int64 `json:"cache_creation_input_tokens"`
		OutputTokens              int64 `json:"output_tokens"`
		OutputTokensDetails       struct {
			ReasoningTokens int64 `json:"reasoning_tokens"`
		} `json:"output_tokens_details"`
	} `json:"usage"`
}

type claudeEventJSON struct {
	Type      string             `json:"type"`
	Timestamp string             `json:"timestamp"`
	Message   *claudeMessageJSON `json:"message"`
}

func loadClaudeRecords(changed map[string]bool) []record {
	records := []record{}
	if changed == nil {
		changed = map[string]bool{}
		walkJSONL(claudeRoot(), func(path string) {
			changed[path] = true
		})
	}
	for path := range changed {
		if !strings.HasSuffix(strings.ToLower(path), ".jsonl") {
			continue
		}
		file, err := os.Open(path)
		if err != nil {
			continue
		}
		scanner := bufio.NewScanner(file)
		scanner.Buffer(make([]byte, 1024*1024), 16*1024*1024)
		lineNumber := 0
		for scanner.Scan() {
			lineNumber++
			var event claudeEventJSON
			if json.Unmarshal(scanner.Bytes(), &event) != nil || event.Message == nil {
				continue
			}
			u := event.Message.Usage
			if u.InputTokens == 0 && u.OutputTokens == 0 {
				continue
			}
			ts := parseTimestamp(event.Timestamp)
			if ts == 0 {
				continue
			}
			model := event.Message.Model
			if model == "" {
				model = "unknown"
			}
			records = append(records, record{
				ThreadID: "claude",
				Agent:    agentClaude,
				Model:    model,
				Key:      path + ":" + strconv.Itoa(lineNumber),
				Path:     path,
				Ts:       ts,
				Date:     dateKey(ts),
				Usage: usage{
					Input:      u.InputTokens,
					Cached:     u.CacheReadInputTokens,
					CacheWrite: u.CacheCreationInputTokens,
					Output:     u.OutputTokens,
					Reasoning:  u.OutputTokensDetails.ReasoningTokens,
					Total:      u.InputTokens + u.OutputTokens,
				},
				ContextWindow: nil,
			})
		}
		file.Close()
	}
	return records
}

type openCodeModelJSON struct {
	ID string `json:"id"`
}

func loadOpenCodeRecords(since int64) []record {
	if _, err := os.Stat(opencodeDB()); err != nil {
		return nil
	}
	db, err := sql.Open("sqlite", "file:"+opencodeDB()+"?mode=ro&_pragma=query_only(1)")
	if err != nil {
		return nil
	}
	defer db.Close()
	rows, err := db.Query(
		`SELECT id, time_updated, model, COALESCE(tokens_input,0), COALESCE(tokens_output,0),
		        COALESCE(tokens_reasoning,0), COALESCE(tokens_cache_read,0), COALESCE(tokens_cache_write,0)
		   FROM session WHERE time_updated > ?`, since)
	if err != nil {
		return nil
	}
	defer rows.Close()

	records := []record{}
	for rows.Next() {
		var id string
		var ts, input, output, reasoning, cacheRead, cacheWrite int64
		var model string
		if err := rows.Scan(&id, &ts, &model, &input, &output, &reasoning, &cacheRead, &cacheWrite); err != nil {
			continue
		}
		if input == 0 && output == 0 {
			continue
		}
		modelName := model
		if strings.HasPrefix(strings.TrimSpace(model), "{") {
			var parsed openCodeModelJSON
			if json.Unmarshal([]byte(model), &parsed) == nil && parsed.ID != "" {
				modelName = parsed.ID
			}
		}
		if modelName == "" {
			modelName = "unknown"
		}
		records = append(records, record{
			ThreadID: "opencode",
			Agent:    agentOpenCode,
			Model:    modelName,
			Key:      id,
			Path:     "opencode",
			Ts:       ts,
			Date:     dateKey(ts),
			Usage: usage{
				Input:      input,
				Cached:     cacheRead,
				CacheWrite: cacheWrite,
				Output:     output,
				Reasoning:  reasoning,
				Total:      input + output,
			},
			ContextWindow: nil,
		})
	}
	return records
}

func newDaySummary(date string) daySummary {
	return daySummary{
		Date:    date,
		ByModel: map[string]*daySummary{},
		ByAgent: map[string]*daySummary{},
	}
}

func addRecord(d *daySummary, r record) {
	d.Input += r.Usage.Input
	d.Cached += r.Usage.Cached
	d.Output += r.Usage.Output
	d.Reasoning += r.Usage.Reasoning
	d.Total += r.Usage.Total
	d.Turns++
	if r.ContextWindow != nil {
		if d.MaxContextWindow == nil || *r.ContextWindow > *d.MaxContextWindow {
			v := *r.ContextWindow
			d.MaxContextWindow = &v
		}
		if *r.ContextWindow > 0 {
			percent := float64(r.Usage.Input) / float64(*r.ContextWindow)
			if d.MaxUsagePercent == nil || percent > *d.MaxUsagePercent {
				p := percent
				d.MaxUsagePercent = &p
			}
		}
	}
	if d.Input > 0 {
		rate := float64(d.Cached) / float64(d.Input)
		d.HitRate = &rate
	}
	if d.ByModel == nil {
		d.ByModel = map[string]*daySummary{}
	}
	md := d.ByModel[r.Model]
	if md == nil {
		s := newDaySummary(r.Model)
		md = &s
		d.ByModel[r.Model] = md
	}
	md.Input += r.Usage.Input
	md.Cached += r.Usage.Cached
	md.Output += r.Usage.Output
	md.Reasoning += r.Usage.Reasoning
	md.Total += r.Usage.Total
	md.Turns++
	if r.ContextWindow != nil && *r.ContextWindow > 0 {
		percent := float64(r.Usage.Input) / float64(*r.ContextWindow)
		if md.MaxUsagePercent == nil || percent > *md.MaxUsagePercent {
			p := percent
			md.MaxUsagePercent = &p
		}
	}
	if md.Input > 0 {
		rate := float64(md.Cached) / float64(md.Input)
		md.HitRate = &rate
	}
	if d.ByAgent == nil {
		d.ByAgent = map[string]*daySummary{}
	}
	ad := d.ByAgent[r.Agent]
	if ad == nil {
		s := newDaySummary(r.Agent)
		ad = &s
		d.ByAgent[r.Agent] = ad
	}
	ad.Input += r.Usage.Input
	ad.Cached += r.Usage.Cached
	ad.Output += r.Usage.Output
	ad.Reasoning += r.Usage.Reasoning
	ad.Total += r.Usage.Total
	ad.Turns++
	if ad.ByModel == nil {
		ad.ByModel = map[string]*daySummary{}
	}
	am := ad.ByModel[r.Model]
	if am == nil {
		s := newDaySummary(r.Model)
		am = &s
		ad.ByModel[r.Model] = am
	}
	am.Input += r.Usage.Input
	am.Cached += r.Usage.Cached
	am.Output += r.Usage.Output
	am.Reasoning += r.Usage.Reasoning
	am.Total += r.Usage.Total
	am.Turns++
	if am.Input > 0 {
		rate := float64(am.Cached) / float64(am.Input)
		am.HitRate = &rate
	}
	if ad.Input > 0 {
		rate := float64(ad.Cached) / float64(ad.Input)
		ad.HitRate = &rate
	}
}

func summarize(records []record, days int) report {
	today := todayKey()
	startMs := time.Now().In(shanghai).AddDate(0, 0, -(days - 1)).UnixMilli()
	startKey := dateKey(startMs)

	byDate := map[string]*daySummary{}
	var inRange []record
	for _, r := range records {
		if r.Date >= startKey && r.Date <= today {
			inRange = append(inRange, r)
			if byDate[r.Date] == nil {
				s := newDaySummary(r.Date)
				byDate[r.Date] = &s
			}
			addRecord(byDate[r.Date], r)
		}
	}

	daily := make([]daySummary, 0, days)
	for offset := 0; offset < days; offset++ {
		key := dateKey(startMs + int64(offset)*86400000)
		if s, ok := byDate[key]; ok {
			daily = append(daily, *s)
		} else {
			daily = append(daily, newDaySummary(key))
		}
	}

	totals := newDaySummary("total")
	for _, r := range inRange {
		addRecord(&totals, r)
	}
	todaySummary := newDaySummary(today)
	if s, ok := byDate[today]; ok {
		todaySummary = *s
	}

	modelTotals := map[string]*daySummary{}
	for _, r := range inRange {
		if modelTotals[r.Model] == nil {
			s := newDaySummary(r.Model)
			modelTotals[r.Model] = &s
		}
		addRecord(modelTotals[r.Model], r)
	}
	models := make([]string, 0, len(modelTotals))
	for model := range modelTotals {
		models = append(models, model)
	}
	sort.Slice(models, func(i, j int) bool {
		return modelTotals[models[i]].Total > modelTotals[models[j]].Total
	})
	agentTotals := map[string]*daySummary{}
	for _, r := range inRange {
		if agentTotals[r.Agent] == nil {
			s := newDaySummary(r.Agent)
			agentTotals[r.Agent] = &s
		}
		addRecord(agentTotals[r.Agent], r)
	}
	agents := make([]string, 0, len(agentTotals))
	for agent := range agentTotals {
		agents = append(agents, agent)
	}
	sort.Slice(agents, func(i, j int) bool {
		return agentTotals[agents[i]].Total > agentTotals[agents[j]].Total
	})

	return report{
		GeneratedAt: time.Now().Format(time.RFC3339),
		Timezone:    shanghaiZone,
		Days:        days,
		RangeStart:  daily[0].Date,
		RangeEnd:    daily[len(daily)-1].Date,
		Totals:      totals,
		Today:       todaySummary,
		Daily:       daily,
		Models:      models,
		Agents:      agents,
	}
}

// DeepSeek Harness JSONL event structures
type deepSeekEvent struct {
	Type string          `json:"type"`
	Seq  int64           `json:"seq"`
	Time int64           `json:"time"`
	Data json.RawMessage `json:"data"`
	ID   string          `json:"id"`
	CreatedAt int64      `json:"createdAt"`
}

type deepSeekContextData struct {
	ContextWindow int64 `json:"contextWindow"`
}

type deepSeekUsageChunk struct {
	Type  string `json:"type"`
	Usage struct {
		InputTokens     int64 `json:"inputTokens"`
		OutputTokens    int64 `json:"outputTokens"`
		CacheReadTokens int64 `json:"cacheReadTokens"`
		CacheWriteTokens int64 `json:"cacheWriteTokens"`
	} `json:"usage"`
}

type deepSeekChunkData struct {
	Turn  int64              `json:"turn"`
	Step  int64              `json:"step"`
	Chunk deepSeekUsageChunk `json:"chunk"`
}

func loadDeepSeekRecords(since int64) []record {
	sessionsDir := filepath.Join(deepSeekHome(), "sessions")
	if _, err := os.Stat(sessionsDir); err != nil {
		return nil
	}

	records := []record{}
	_ = filepath.Walk(sessionsDir, func(path string, info os.FileInfo, err error) error {
		if err != nil || info.IsDir() {
			return nil
		}
		if !strings.HasSuffix(strings.ToLower(info.Name()), ".jsonl.zstd") {
			return nil
		}
		sessionRecords := loadDeepSeekSession(path, since)
		records = append(records, sessionRecords...)
		return nil
	})
	return records
}

func loadDeepSeekSession(path string, since int64) []record {
	file, err := os.Open(path)
	if err != nil {
		return nil
	}
	defer file.Close()

	// Read and decompress zstd
	dctx, err := zstd.NewReader(file)
	if err != nil {
		return nil
	}
	defer dctx.Close()

	// Read decompressed data
	var buf []byte
	buf, err = io.ReadAll(dctx)
	if err != nil {
		return nil
	}

	var sessionID string
	var createdAt int64
	var contextWindow *int64
	var model string
	var records []record

	scanner := bufio.NewScanner(bytes.NewReader(buf))
	scanner.Buffer(make([]byte, 1024*1024), 16*1024*1024)

	for scanner.Scan() {
		line := scanner.Bytes()
		var event deepSeekEvent
		if json.Unmarshal(line, &event) != nil {
			continue
		}

		switch event.Type {
		case "session":
			sessionID = event.ID
			createdAt = event.CreatedAt

		case "request/context":
			var ctx deepSeekContextData
			if json.Unmarshal(event.Data, &ctx) == nil && ctx.ContextWindow > 0 {
				contextWindow = &ctx.ContextWindow
			}

		case "request/header":
			// Extract model name from header
			var headerData struct {
				Header struct {
					Config struct {
						Model string `json:"model"`
					} `json:"config"`
				} `json:"header"`
			}
			if json.Unmarshal(event.Data, &headerData) == nil && headerData.Header.Config.Model != "" {
				model = headerData.Header.Config.Model
			}

		case "assistant/chunk":
			var chunkData deepSeekChunkData
			if json.Unmarshal(event.Data, &chunkData) != nil || chunkData.Chunk.Type != "usage" {
				continue
			}
			ts := event.Time
			if ts == 0 || ts < since || sessionID == "" {
				continue
			}
			u := chunkData.Chunk.Usage
			if u.InputTokens == 0 && u.OutputTokens == 0 {
				continue
			}
			modelName := model
			if modelName == "" {
				modelName = "unknown"
			}
			// DSH 的 inputTokens 仅含未缓存输入，cacheRead 单独计。
			// 与其他 Agent（ZCode/Claude）口径一致：输入含缓存读取，命中率才有意义。
			inputTotal := u.InputTokens + u.CacheReadTokens
			records = append(records, record{
				ThreadID: sessionID,
				Agent:    agentDeepSeek,
				Model:    modelName,
				Key:      "dsh-" + sessionID + "-" + strconv.FormatInt(event.Seq, 10),
				Path:     path,
				Ts:       ts,
				Date:     dateKey(ts),
				Usage: usage{
					Input:      inputTotal,
					Cached:     u.CacheReadTokens,
					CacheWrite: u.CacheWriteTokens,
					Output:     u.OutputTokens,
					Reasoning:  0,
					Total:      inputTotal + u.OutputTokens,
				},
				ContextWindow: contextWindow,
			})
		}
	}

	if sessionID == "" || createdAt == 0 {
		return nil
	}
	return records
}

func collect(days int, agent string) report {
	if err := ensureCached(agent); err == nil {
		return summarize(loadCacheRecords(agent), days)
	}

	var records []record
	if agent == agentAll || agent == agentCodex {
		codexRecords, _ := loadCodexRecords(nil, 0)
		records = append(records, codexRecords...)
	}
	if agent == agentAll || agent == agentZcode {
		zcodeRecords, _ := loadZCodeRecords(zcodeDB(), 0)
		records = append(records, zcodeRecords...)
	}
	if agent == agentAll || agent == agentClaude {
		records = append(records, loadClaudeRecords(nil)...)
	}
	if agent == agentAll || agent == agentOpenCode {
		records = append(records, loadOpenCodeRecords(0)...)
	}
	if agent == agentAll || agent == agentDeepSeek {
		records = append(records, loadDeepSeekRecords(0)...)
	}
	return summarize(records, days)
}
