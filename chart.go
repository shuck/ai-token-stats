package main

import (
	"fmt"

	"github.com/lxn/walk"
)

func formatTokens(value int64) string {
	if value >= 100000000 {
		return fmt.Sprintf("%.2f亿", float64(value)/100000000)
	}
	if value >= 10000 {
		return fmt.Sprintf("%.2f万", float64(value)/10000)
	}
	return fmt.Sprintf("%d", value)
}

func formatPercent(value *float64) string {
	if value == nil {
		return "无数据"
	}
	return fmt.Sprintf("%.1f%%", *value*100)
}

func formatContextWindow(value *int64) string {
	if value == nil {
		return "无数据"
	}
	return formatTokens(*value)
}

// displayName 返回 Agent 的界面显示名（DeepSeek 显示为 DSH）。
func displayName(agent string) string {
	if agent == agentDeepSeek {
		return "DSH"
	}
	return agent
}

func (a *app) stackKeys() ([]string, bool) {
	if a.agent == "" || a.agent == agentAll {
		return a.data.Agents, true
	}
	return a.data.Models, false
}

func drawCentered(canvas *walk.Canvas, text string, rect walk.Rectangle, font *walk.Font, color walk.Color) error {
	return canvas.DrawTextPixels(
		text,
		font,
		color,
		rect,
		walk.TextCenter|walk.TextVCenter|walk.TextSingleLine|walk.TextWordEllipsis,
	)
}

type chartGeometry struct {
	margin       int
	cardGap      int
	cardWidth    int
	cardHeight   int
	summaryBottom int
	chartTitleY  int
	plotLeft     int
	plotRight    int
	plotTop      int
	plotBottom   int
	plotWidth    int
	plotHeight   int
	slot         int
	barWidth     int
	labelStep    int
	days         int
}

func computeGeometry(width, height, days int) chartGeometry {
	margin := 20
	cardGap := 8
	cardHeight := 58
	cardWidth := 0
	if width > margin*2+cardGap*4 {
		cardWidth = (width - 2*margin - 4*cardGap) / 5
	}
	summaryBottom := margin + cardHeight
	chartTitleY := summaryBottom + 12
	plotLeft := 24
	plotRight := width - 24
	plotTop := chartTitleY + 26 + 8
	plotBottom := height - 46
	plotWidth := plotRight - plotLeft
	plotHeight := plotBottom - plotTop
	if plotWidth < 1 {
		plotWidth = 1
	}
	if plotHeight < 1 {
		plotHeight = 1
	}
	slot := 1
	barWidth := 1
	labelStep := 1
	if days > 0 {
		slot = plotWidth / days
		if slot < 1 {
			slot = 1
		}
		barWidth = slot * 55 / 100
		if barWidth < 1 {
			barWidth = 1
		}
		if days > 15 {
			labelStep = (days + 14) / 15
		}
	}
	return chartGeometry{
		margin:        margin,
		cardGap:       cardGap,
		cardWidth:     cardWidth,
		cardHeight:    cardHeight,
		summaryBottom: summaryBottom,
		chartTitleY:   chartTitleY,
		plotLeft:      plotLeft,
		plotRight:     plotRight,
		plotTop:       plotTop,
		plotBottom:    plotBottom,
		plotWidth:     plotWidth,
		plotHeight:    plotHeight,
		slot:          slot,
		barWidth:      barWidth,
		labelStep:     labelStep,
		days:          days,
	}
}

func (g chartGeometry) indexAt(x int) int {
	if g.days <= 0 || x < g.plotLeft || x >= g.plotRight {
		return -1
	}
	index := (x - g.plotLeft) / g.slot
	if index < 0 || index >= g.days {
		return -1
	}
	return index
}

func (g chartGeometry) barRect(index int) walk.Rectangle {
	x := g.plotLeft + index*g.slot + (g.slot-g.barWidth)/2
	return walk.Rectangle{X: x, Y: g.plotTop, Width: g.barWidth, Height: g.plotBottom - g.plotTop}
}

func maxTokenInTenThousand(daily []daySummary) float64 {
	max := 1.0
	for _, d := range daily {
		v := float64(d.Total) / 10000
		if v > max {
			max = v
		}
	}
	return max
}

func (a *app) paintChart(canvas *walk.Canvas, bounds walk.Rectangle) error {
	if err := canvas.GradientFillRectanglePixels(walk.RGB(232, 242, 252), walk.RGB(255, 255, 255), walk.Vertical, bounds); err != nil {
		return err
	}

	titleFont, err := walk.NewFont("Microsoft YaHei", 12, walk.FontBold)
	if err != nil {
		return err
	}
	textFont, err := walk.NewFont("Microsoft YaHei", 9, 0)
	if err != nil {
		return err
	}
	valueFont, err := walk.NewFont("Microsoft YaHei", 11, walk.FontBold)
	if err != nil {
		return err
	}

	margin := 20
	if a.data == nil {
		return drawCentered(canvas, "无数据", walk.Rectangle{X: bounds.X, Y: bounds.Y, Width: bounds.Width, Height: bounds.Height}, titleFont, walk.RGB(120, 120, 120))
	}

	g := computeGeometry(bounds.Width, bounds.Height, len(a.data.Daily))

	cards := []struct {
		title string
		value string
	}{
		{fmt.Sprintf("最近 %d 天", a.data.Days), formatTokens(a.data.Totals.Total)},
		{"今日", formatTokens(a.data.Today.Total)},
		{"总命中率", formatPercent(a.data.Totals.HitRate)},
		{"今日命中率", formatPercent(a.data.Today.HitRate)},
		{"今日上下文峰值", formatPercent(a.data.Today.MaxUsagePercent)},
	}
	lightBrush, err := walk.NewSolidColorBrush(walk.RGB(250, 252, 255))
	if err != nil {
		return err
	}
	defer lightBrush.Dispose()
	cardPen, err := walk.NewCosmeticPen(walk.PenSolid, walk.RGB(210, 224, 240))
	if err != nil {
		return err
	}
	defer cardPen.Dispose()
	textColor := walk.RGB(60, 60, 60)
	valueColor := walk.RGB(20, 90, 220)
	cardEllipse := walk.Size{Width: 10, Height: 10}

	for i, card := range cards {
		rect := walk.Rectangle{
			X:      bounds.X + margin + i*(g.cardWidth+g.cardGap),
			Y:      bounds.Y + margin,
			Width:  g.cardWidth,
			Height: g.cardHeight,
		}
		if err := canvas.FillRoundedRectanglePixels(lightBrush, rect, cardEllipse); err != nil {
			return err
		}
		if err := canvas.DrawRoundedRectanglePixels(cardPen, rect, cardEllipse); err != nil {
			return err
		}
		titleRect := walk.Rectangle{X: rect.X, Y: rect.Y + 4, Width: rect.Width, Height: 20}
		valueRect := walk.Rectangle{X: rect.X, Y: rect.Y + 26, Width: rect.Width, Height: 26}
		if err := drawCentered(canvas, card.title, titleRect, textFont, textColor); err != nil {
			return err
		}
		if err := drawCentered(canvas, card.value, valueRect, valueFont, valueColor); err != nil {
			return err
		}
	}

	chartTitleRect := walk.Rectangle{
		X:      bounds.X,
		Y:      bounds.Y + g.chartTitleY,
		Width:  bounds.Width,
		Height: 26,
	}
	if err := drawCentered(canvas, "AI Token 统计", chartTitleRect, titleFont, walk.RGB(30, 30, 30)); err != nil {
		return err
	}

	plotLeft := bounds.X + g.plotLeft
	plotBottom := bounds.Y + g.plotBottom

	days := len(a.data.Daily)
	if days == 0 {
		return nil
	}
	maxToken := maxTokenInTenThousand(a.data.Daily)
	palette := []walk.Color{
		walk.RGB(20, 120, 230),
		walk.RGB(0, 180, 150),
		walk.RGB(150, 100, 220),
		walk.RGB(240, 140, 30),
		walk.RGB(70, 170, 70),
		walk.RGB(230, 70, 120),
		walk.RGB(140, 140, 140),
	}
	stackKeys, stackByAgent := a.stackKeys()
	if len(stackKeys) == 0 {
		return nil
	}

	for i, d := range a.data.Daily {
		x := plotLeft + i*g.slot + (g.slot-g.barWidth)/2
		cumulative := 0
		for keyIndex, key := range stackKeys {
			var segmentTotal int64
			if stackByAgent {
				if ad := d.ByAgent[key]; ad != nil {
					segmentTotal = ad.Total
				}
			} else if md := d.ByModel[key]; md != nil {
				segmentTotal = md.Total
			}
			if segmentTotal <= 0 {
				continue
			}
			segmentHeight := int(float64(segmentTotal)/10000 / maxToken * float64(g.plotHeight))
			if segmentHeight <= 0 {
				continue
			}
			brush, err := walk.NewSolidColorBrush(palette[keyIndex%len(palette)])
			if err != nil {
				return err
			}
			barRect := walk.Rectangle{X: x, Y: plotBottom - cumulative - segmentHeight, Width: g.barWidth, Height: segmentHeight}
			if err := canvas.FillRectanglePixels(brush, barRect); err != nil {
				brush.Dispose()
				return err
			}
			brush.Dispose()
			cumulative += segmentHeight
		}
		if i%g.labelStep == 0 {
			dateLabel := d.Date
			if len(dateLabel) > 5 {
				dateLabel = dateLabel[5:]
			}
			labelRect := walk.Rectangle{X: plotLeft + i*g.slot, Y: plotBottom + 4, Width: g.slot, Height: 16}
			if err := drawCentered(canvas, dateLabel, labelRect, textFont, textColor); err != nil {
				return err
			}
		}
	}

	plotRight := bounds.X + g.plotRight
	legendX := plotLeft
	legendY := plotBottom + 24
	for keyIndex, key := range stackKeys {
		colorRect := walk.Rectangle{X: legendX, Y: legendY + 2, Width: 10, Height: 10}
		brush, err := walk.NewSolidColorBrush(palette[keyIndex%len(palette)])
		if err == nil {
			_ = canvas.FillRectanglePixels(brush, colorRect)
			brush.Dispose()
		}
		textRect := walk.Rectangle{X: legendX + 14, Y: legendY, Width: 116, Height: 16}
		_ = drawCentered(canvas, displayName(key), textRect, textFont, textColor)
		legendX += 130
		if legendX+130 > plotRight {
			legendX = plotLeft
			legendY += 18
		}
	}

	if a.hoverActive && a.hoverIndex >= 0 && a.hoverIndex < days {
		hoverPen, err := walk.NewCosmeticPen(walk.PenSolid, walk.RGB(255, 140, 0))
		if err == nil {
			barTop := plotBottom
			total := a.data.Daily[a.hoverIndex].Total
			if total > 0 {
				barHeight := int(float64(total)/10000 / maxToken * float64(g.plotHeight))
				barTop = plotBottom - barHeight
			}
			rect := walk.Rectangle{
				X:      plotLeft + a.hoverIndex*g.slot + (g.slot-g.barWidth)/2,
				Y:      barTop,
				Width:  g.barWidth,
				Height: plotBottom - barTop,
			}
			_ = canvas.DrawRectanglePixels(hoverPen, rect)
			hoverPen.Dispose()
		}

		day := a.data.Daily[a.hoverIndex]
		tipWidth := 320
		lines := []string{
			fmt.Sprintf("%s", day.Date),
			fmt.Sprintf("总 token：%s", formatTokens(day.Total)),
			fmt.Sprintf("输入：%s | 缓存：%s", formatTokens(day.Input), formatTokens(day.Cached)),
			fmt.Sprintf("输出：%s | 推理：%s", formatTokens(day.Output), formatTokens(day.Reasoning)),
			fmt.Sprintf("轮次：%d | 上下文：%s", day.Turns, formatContextWindow(day.MaxContextWindow)),
			fmt.Sprintf("使用率峰值：%s | 命中率：%s", formatPercent(day.MaxUsagePercent), formatPercent(day.HitRate)),
		}
		if stackByAgent {
			for _, agent := range a.data.Agents {
				if ad := day.ByAgent[agent]; ad != nil && ad.Total > 0 {
					lines = append(lines, fmt.Sprintf("%s：%s（命中率 %s）", displayName(agent), formatTokens(ad.Total), formatPercent(ad.HitRate)))
					for _, model := range a.data.Models {
						if md := ad.ByModel[model]; md != nil && md.Total > 0 {
							lines = append(lines, fmt.Sprintf("  %s：%s（命中率 %s）", model, formatTokens(md.Total), formatPercent(md.HitRate)))
						}
					}
				}
			}
		} else {
			for _, model := range a.data.Models {
				if md := day.ByModel[model]; md != nil && md.Total > 0 {
					lines = append(lines, fmt.Sprintf("%s：%s（命中率 %s）", model, formatTokens(md.Total), formatPercent(md.HitRate)))
				}
			}
		}
		tipHeight := 10 + len(lines)*19
		tipX := a.hoverX + 14
		tipY := a.hoverY - tipHeight - 10
		if tipX+tipWidth > bounds.X+bounds.Width-8 {
			tipX = bounds.X + bounds.Width - tipWidth - 8
		}
		if tipX < bounds.X+8 {
			tipX = bounds.X + 8
		}
		if tipY < bounds.Y+8 {
			tipY = bounds.Y + 8
		}
		if tipY+tipHeight > bounds.Y+bounds.Height-8 {
			tipY = bounds.Y + bounds.Height - tipHeight - 8
		}
		tipRect := walk.Rectangle{X: tipX, Y: tipY, Width: tipWidth, Height: tipHeight}
		tipBrush, err := walk.NewSolidColorBrush(walk.RGB(255, 253, 247))
		if err == nil {
			_ = canvas.FillRectanglePixels(tipBrush, tipRect)
			tipBrush.Dispose()
		}
		tipPen, err := walk.NewCosmeticPen(walk.PenSolid, walk.RGB(200, 160, 90))
		if err == nil {
			_ = canvas.DrawRectanglePixels(tipPen, tipRect)
			tipPen.Dispose()
		}

		for i, line := range lines {
			lineRect := walk.Rectangle{X: tipX + 10, Y: tipY + 4 + i*19, Width: tipWidth - 20, Height: 18}
			if i == 0 {
				_ = canvas.DrawTextPixels(line, valueFont, walk.RGB(20, 90, 220), lineRect, walk.TextLeft|walk.TextTop|walk.TextSingleLine)
			} else {
				_ = canvas.DrawTextPixels(line, textFont, textColor, lineRect, walk.TextLeft|walk.TextTop|walk.TextSingleLine)
			}
		}
	}

	return nil
}
