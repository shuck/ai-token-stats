package main

import (
	"testing"
	"time"
)

func TestClickDetector(t *testing.T) {
	d := &clickDetector{}
	base := time.Now()

	if d.onMouseUp(base) {
		t.Fatal("first click should not open")
	}
	if !d.onMouseUp(base.Add(100 * time.Millisecond)) {
		t.Fatal("double-click within interval should open")
	}
	if d.onMouseUp(base.Add(200 * time.Millisecond)) {
		t.Fatal("click right after opening should not open again")
	}
	if d.onMouseUp(base.Add(800 * time.Millisecond)) {
		t.Fatal("slow double-click outside interval should not open")
	}
	if !d.onMouseUp(base.Add(1000 * time.Millisecond)) {
		t.Fatal("fast follow-up after a slow click should open")
	}
}
