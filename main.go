package main

import (
	"errors"
	"fmt"
	"image"
	"image/color"
	"image/draw"
	"os"
	"strconv"
	"time"

	"github.com/lxn/walk"
	"github.com/lxn/win"
	"golang.org/x/sys/windows"
)

type app struct {
	mw     *walk.MainWindow
	ni     *walk.NotifyIcon
	combo  *walk.ComboBox
	agentCombo *walk.ComboBox
	chart  *walk.CustomWidget
	days   int
	agent  string
	data   *report
	exiting bool
	smoke  bool
	hoverActive bool
	hoverIndex  int
	hoverX      int
	hoverY      int
	cfg         *config
	scanning    bool
	clicks      clickDetector
}

const doubleClickInterval = 500 * time.Millisecond

// clickDetector decides whether two left-clicks form a double-click.
// walk's NotifyIcon does not dispatch WM_LBUTTONDBLCLK, so the second press
// of a double-click never fires MouseDown; both mouse-up messages are
// delivered though, so detection runs on MouseUp.
type clickDetector struct {
	last time.Time
}

func (d *clickDetector) onMouseUp(now time.Time) bool {
	if !d.last.IsZero() && now.Sub(d.last) <= doubleClickInterval {
		d.last = time.Time{}
		return true
	}
	d.last = now
	return false
}

func makeIcon() (*walk.Icon, error) {
	img := image.NewRGBA(image.Rect(0, 0, 32, 32))
	draw.Draw(img, img.Bounds(), &image.Uniform{C: color.RGBA{R: 20, G: 90, B: 220, A: 255}}, image.Point{}, draw.Src)
	white := &image.Uniform{C: color.RGBA{R: 255, G: 255, B: 255, A: 255}}
	light := &image.Uniform{C: color.RGBA{R: 190, G: 220, B: 255, A: 255}}
	draw.Draw(img, image.Rect(5, 12, 12, 27), light, image.Point{}, draw.Src)
	draw.Draw(img, image.Rect(13, 6, 20, 27), white, image.Point{}, draw.Src)
	draw.Draw(img, image.Rect(21, 16, 28, 27), light, image.Point{}, draw.Src)
	return walk.NewIconFromImage(img)
}

func acquireSingleInstance() (windows.Handle, bool, error) {
	name, err := windows.UTF16PtrFromString(`Global\AITokenStatsTray`)
	if err != nil {
		return 0, false, err
	}
	handle, err := windows.CreateMutex(nil, false, name)
	if err != nil {
		if errors.Is(err, windows.ERROR_ALREADY_EXISTS) {
			return 0, false, nil
		}
		return 0, false, err
	}
	return handle, true, nil
}

func (a *app) refresh() {
	a.ensurePaths()
	if a.agent == "" {
		a.agent = agentAll
	}
	r := collect(a.days, a.agent)
	a.data = &r
	if a.chart != nil {
		_ = a.chart.Invalidate()
	}
	if a.ni != nil {
		_ = a.ni.SetToolTip(fmt.Sprintf("AI Token 统计 | 今日 %s | 命中 %s", formatTokens(r.Today.Total), formatPercent(r.Today.HitRate)))
	}
}

func (a *app) ensurePaths() {
	needScan := false
	for _, agent := range allAgents {
		if !pathExists(agentPaths[agent]) {
			needScan = true
			break
		}
	}
	if !needScan {
		return
	}
	if a.smoke {
		a.scanAll(false)
		return
	}
	if a.scanning {
		return
	}
	a.scanning = true
	go func() {
		defer func() { a.scanning = false }()
		changed := a.scanAll(false)
		if a.mw != nil {
			a.mw.Synchronize(func() {
				if changed && a.ni != nil {
					_ = a.ni.ShowMessage("AI Token 统计", "Agent 数据路径已自动更新。")
				}
				a.refresh()
			})
		}
	}()
}

func (a *app) scanAll(force bool) bool {
	changed := false
	roots := scanRoots()
	for _, agent := range allAgents {
		if !force && pathExists(agentPaths[agent]) {
			continue
		}
		if p := discoverAgentPath(agent, roots); p != "" {
			if p != agentPaths[agent] {
				agentPaths[agent] = p
				a.cfg.Agents[agent] = agentPath{Path: p, DetectedAt: time.Now().Format(time.RFC3339)}
				changed = true
			}
		}
	}
	if changed {
		if err := saveConfig(configPath, a.cfg); err != nil {
			fmt.Fprintln(os.Stderr, "save config:", err)
		}
	}
	return changed
}

func (a *app) showWindow() {
	if a.mw == nil {
		return
	}
	hwnd := a.mw.Handle()
	if win.IsIconic(hwnd) {
		win.ShowWindow(hwnd, win.SW_RESTORE)
	}
	a.mw.Show()
	a.mw.SetVisible(true)
	// Show/SetVisible do not raise Z-order or activate an already visible
	// window, so bring it to the foreground explicitly.
	win.SetForegroundWindow(hwnd)
}

func (a *app) run() error {
	mw, err := walk.NewMainWindow()
	if err != nil {
		return err
	}
	a.mw = mw
	a.days = 30

	if err := mw.SetTitle("AI Token 统计"); err != nil {
		return err
	}
	if err := mw.SetClientSize(walk.Size{Width: 900, Height: 600}); err != nil {
		return err
	}
	bg, err := walk.NewVerticalGradientBrush([]walk.GradientStop{
		{Offset: 0, Color: walk.RGB(232, 242, 252)},
		{Offset: 1, Color: walk.RGB(255, 255, 255)},
	})
	if err != nil {
		return err
	}
	mw.SetBackground(bg)

	root := walk.NewVBoxLayout()
	if err := mw.SetLayout(root); err != nil {
		return err
	}

	top, err := walk.NewComposite(mw)
	if err != nil {
		return err
	}
	if err := top.SetLayout(walk.NewHBoxLayout()); err != nil {
		return err
	}

	label, err := walk.NewLabel(top)
	if err != nil {
		return err
	}
	_ = label.SetText("最近天数:")
	font, _ := walk.NewFont("Microsoft YaHei", 10, 0)
	label.SetFont(font)

	combo, err := walk.NewComboBox(top)
	if err != nil {
		return err
	}
	if err := combo.SetModel([]string{"7", "14", "30", "90"}); err != nil {
		return err
	}
	if err := combo.SetCurrentIndex(2); err != nil {
		return err
	}
	combo.SetFont(font)
	a.combo = combo

	agentLabel, err := walk.NewLabel(top)
	if err != nil {
		return err
	}
	_ = agentLabel.SetText("Agent:")
	agentLabel.SetFont(font)

	agentCombo, err := walk.NewComboBox(top)
	if err != nil {
		return err
	}
	if err := agentCombo.SetModel([]string{"全部", "Codex", "ZCode", "Claude", "OpenCode"}); err != nil {
		return err
	}
	if err := agentCombo.SetCurrentIndex(0); err != nil {
		return err
	}
	agentCombo.SetFont(font)
	a.agentCombo = agentCombo

	refreshButton, err := walk.NewPushButton(top)
	if err != nil {
		return err
	}
	_ = refreshButton.SetText("刷新")
	refreshButton.SetFont(font)

	chart, err := walk.NewCustomWidgetPixels(mw, 0, a.paintChart)
	if err != nil {
		return err
	}
	chart.SetPaintMode(walk.PaintBuffered)
	a.chart = chart
	if err := root.SetStretchFactor(chart, 4); err != nil {
		return err
	}
	chart.MouseMove().Attach(func(x, y int, button walk.MouseButton) {
		if a.data == nil {
			return
		}
		width := chart.Size().Width
		height := chart.Size().Height
		g := computeGeometry(width, height, len(a.data.Daily))
		index := g.indexAt(x)
		a.hoverActive = index >= 0
		a.hoverIndex = index
		a.hoverX = x
		a.hoverY = y
		_ = chart.Invalidate()
	})

	icon, err := makeIcon()
	if err != nil {
		return err
	}
	ni, err := walk.NewNotifyIcon(mw)
	if err != nil {
		return err
	}
	a.ni = ni
	if err := ni.SetIcon(icon); err != nil {
		return err
	}
	if err := ni.SetToolTip("Codex 用量"); err != nil {
		return err
	}
	if err := ni.SetVisible(true); err != nil {
		return err
	}
	ni.MouseUp().Attach(func(x, y int, button walk.MouseButton) {
		if button == walk.LeftButton && a.clicks.onMouseUp(time.Now()) {
			a.showWindow()
		}
	})

	openAction := walk.NewAction()
	_ = openAction.SetText("打开面板")
	openAction.Triggered().Attach(a.showWindow)
	refreshAction := walk.NewAction()
	_ = refreshAction.SetText("刷新")
	refreshAction.Triggered().Attach(a.refresh)
	rescanAction := walk.NewAction()
	_ = rescanAction.SetText("重新扫描路径")
	rescanAction.Triggered().Attach(func() {
		if a.scanning {
			return
		}
		a.scanning = true
		go func() {
			defer func() { a.scanning = false }()
			changed := a.scanAll(true)
			if a.mw != nil {
				a.mw.Synchronize(func() {
					if a.ni != nil {
						if changed {
							_ = a.ni.ShowMessage("AI Token 统计", "Agent 数据路径已更新。")
						} else {
							_ = a.ni.ShowMessage("AI Token 统计", "未发现新的 Agent 数据路径。")
						}
					}
					a.refresh()
				})
			}
		}()
	})
	settingsAction := walk.NewAction()
	_ = settingsAction.SetText("设置 Agent 路径…")
	settingsAction.Triggered().Attach(func() { a.showSettingsDialog() })
	exitAction := walk.NewAction()
	_ = exitAction.SetText("退出")
	exitAction.Triggered().Attach(func() {
		a.exiting = true
		_ = ni.Dispose()
		walk.App().Exit(0)
	})
	menu := ni.ContextMenu().Actions()
	if err := menu.Add(openAction); err != nil {
		return err
	}
	if err := menu.Add(refreshAction); err != nil {
		return err
	}
	if err := menu.Add(rescanAction); err != nil {
		return err
	}
	if err := menu.Add(settingsAction); err != nil {
		return err
	}
	if err := menu.Add(exitAction); err != nil {
		return err
	}

	refreshButton.Clicked().Attach(func() {
		value, err := strconv.Atoi(a.combo.Text())
		if err == nil {
			a.days = value
		}
		a.updateAgent()
		a.refresh()
	})
	a.combo.CurrentIndexChanged().Attach(func() {
		value, err := strconv.Atoi(a.combo.Text())
		if err == nil {
			a.days = value
		}
		a.refresh()
	})
	a.agentCombo.CurrentIndexChanged().Attach(func() {
		a.updateAgent()
		a.refresh()
	})

	mw.Closing().Attach(func(canceled *bool, reason walk.CloseReason) {
		if !a.exiting {
			*canceled = true
			mw.Hide()
		}
	})

	go func() {
		ticker := time.NewTicker(time.Minute)
		defer ticker.Stop()
		for range ticker.C {
			mw.Synchronize(a.refresh)
		}
	}()

	a.refresh()
	if a.smoke {
		fmt.Printf("SMOKE OK: days=%d turns=%d agents=%v models=%v\n", a.data.Days, a.data.Totals.Turns, a.data.Agents, a.data.Models)
		for _, model := range a.data.Models {
			if md := a.data.Totals.ByModel[model]; md != nil {
				fmt.Printf("  %s: total=%d input=%d cached=%d\n", model, md.Total, md.Input, md.Cached)
			}
		}
		for _, agent := range a.data.Agents {
			if ad := a.data.Totals.ByAgent[agent]; ad != nil {
				fmt.Printf("  [%s] total=%d turns=%d\n", agent, ad.Total, ad.Turns)
			}
		}
		_ = ni.Dispose()
		os.Exit(0)
	}
	a.showWindow()
	_ = mw.Run()
	return nil
}

func (a *app) updateAgent() {
	if a.agentCombo == nil {
		return
	}
	switch a.agentCombo.Text() {
	case "Codex":
		a.agent = agentCodex
	case "ZCode":
		a.agent = agentZcode
	case "Claude":
		a.agent = agentClaude
	case "OpenCode":
		a.agent = agentOpenCode
	default:
		a.agent = agentAll
	}
}

func main() {
	mutex, acquired, err := acquireSingleInstance()
	if err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(1)
	}
	if !acquired {
		return
	}
	defer windows.CloseHandle(mutex)

	cfg, err := initPaths()
	if err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(1)
	}

	hold := false
	a := &app{cfg: cfg}
	for _, arg := range os.Args[1:] {
		if arg == "-smoke" {
			a.smoke = true
		} else if arg == "-hold" {
			hold = true
		}
	}
	if hold {
		time.Sleep(5 * time.Second)
		return
	}
	if err := a.run(); err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(1)
	}
}
