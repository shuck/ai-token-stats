package main

import (
	"fmt"
	"os"
	"path/filepath"
	"sync"
	"time"
)

var (
	logMu   sync.Mutex
	logPath string
)

func initLog(dir string) {
	logPath = filepath.Join(dir, "ai-token-stats.log")
	logMsg("log initialized")
}

func logMsg(format string, args ...interface{}) {
	logMu.Lock()
	defer logMu.Unlock()
	if logPath == "" {
		return
	}
	line := fmt.Sprintf("[%s] %s\n", time.Now().Format("2006-01-02 15:04:05"), fmt.Sprintf(format, args...))
	f, err := os.OpenFile(logPath, os.O_CREATE|os.O_APPEND|os.O_WRONLY, 0o644)
	if err != nil {
		return
	}
	defer f.Close()
	_, _ = f.WriteString(line)
}
