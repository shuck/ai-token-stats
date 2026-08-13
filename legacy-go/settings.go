package main

import (
	"time"

	"github.com/lxn/walk"
)

type pathRow struct {
	label *walk.Label
	edit  *walk.LineEdit
	agent string
	isDir bool
}

func (a *app) showSettingsDialog() {
	if a.mw == nil {
		return
	}
	dlg, err := walk.NewDialogWithFixedSize(a.mw)
	if err != nil {
		return
	}
	_ = dlg.SetTitle("设置 Agent 路径")
	if err := dlg.SetClientSize(walk.Size{Width: 640, Height: 280}); err != nil {
		return
	}
	root := walk.NewVBoxLayout()
	if err := dlg.SetLayout(root); err != nil {
		return
	}

	var rows []*pathRow
	for _, def := range []struct {
		agent string
		label string
		isDir bool
	}{
		{agentCodex, "Codex home 目录", true},
		{agentZcode, "ZCode db.sqlite", false},
		{agentClaude, "Claude projects 目录", true},
		{agentOpenCode, "OpenCode opencode.db", false},
	} {
		comp, err := walk.NewComposite(dlg)
		if err != nil {
			return
		}
		h := walk.NewHBoxLayout()
		_ = comp.SetLayout(h)
		lbl, err := walk.NewLabel(comp)
		if err != nil {
			return
		}
		_ = lbl.SetText(def.label)
		edit, err := walk.NewLineEdit(comp)
		if err != nil {
			return
		}
		_ = edit.SetText(agentPaths[def.agent])
		_ = h.SetStretchFactor(edit, 3)
		browse, err := walk.NewPushButton(comp)
		if err != nil {
			return
		}
		_ = browse.SetText("浏览…")
		r := &pathRow{label: lbl, edit: edit, agent: def.agent, isDir: def.isDir}
		browse.Clicked().Attach(func() {
			r.pickPath(a.mw)
		})
		rows = append(rows, r)
	}

	btnComp, err := walk.NewComposite(dlg)
	if err != nil {
		return
	}
	hb := walk.NewHBoxLayout()
	_ = btnComp.SetLayout(hb)
	okBtn, err := walk.NewPushButton(btnComp)
	if err != nil {
		return
	}
	_ = okBtn.SetText("确定")
	okBtn.Clicked().Attach(func() {
		for _, r := range rows {
			p := r.edit.Text()
			if p == "" {
				continue
			}
			if !validateAgentPath(r.agent, p) {
				walk.MsgBox(dlg, "路径无效", r.label.Text()+" 不存在或不是有效数据源。", walk.MsgBoxIconError)
				return
			}
			agentPaths[r.agent] = p
			a.cfg.Agents[r.agent] = agentPath{Path: p, DetectedAt: time.Now().Format(time.RFC3339)}
		}
		if err := saveConfig(configPath, a.cfg); err != nil {
			walk.MsgBox(dlg, "保存失败", err.Error(), walk.MsgBoxIconError)
			return
		}
		dlg.Accept()
		a.refresh()
	})
	cancelBtn, err := walk.NewPushButton(btnComp)
	if err != nil {
		return
	}
	_ = cancelBtn.SetText("取消")
	cancelBtn.Clicked().Attach(func() {
		dlg.Cancel()
	})
	_ = dlg.Run()
}

func (r *pathRow) pickPath(owner walk.Form) {
	var dlg walk.FileDialog
	dlg.Title = "选择路径"
	dlg.FilePath = r.edit.Text()
	if r.isDir {
		if accepted, _ := dlg.ShowBrowseFolder(owner); accepted && dlg.FilePath != "" {
			_ = r.edit.SetText(dlg.FilePath)
		}
		return
	}
	if accepted, _ := dlg.ShowOpen(owner); accepted && dlg.FilePath != "" {
		_ = r.edit.SetText(dlg.FilePath)
	}
}
