// Package realtime provides a small in-process publish/subscribe hub used to
// fan out server-status updates to SSE clients. This is single-instance only;
// a horizontally-scaled deployment would back this with Postgres LISTEN/NOTIFY
// or Redis pub/sub behind the same interface.
package realtime

import "sync"

// Hub fans messages out to subscribers grouped by topic.
type Hub struct {
	mu   sync.RWMutex
	subs map[string]map[chan []byte]struct{}
}

// NewHub creates an empty Hub.
func NewHub() *Hub {
	return &Hub{subs: make(map[string]map[chan []byte]struct{})}
}

// Subscribe registers a buffered channel for a topic and returns it along with
// an unsubscribe function the caller must defer.
func (h *Hub) Subscribe(topic string) (<-chan []byte, func()) {
	ch := make(chan []byte, 16)
	h.mu.Lock()
	if h.subs[topic] == nil {
		h.subs[topic] = make(map[chan []byte]struct{})
	}
	h.subs[topic][ch] = struct{}{}
	h.mu.Unlock()

	var once sync.Once
	cancel := func() {
		once.Do(func() {
			h.mu.Lock()
			if set, ok := h.subs[topic]; ok {
				delete(set, ch)
				if len(set) == 0 {
					delete(h.subs, topic)
				}
			}
			h.mu.Unlock()
			close(ch)
		})
	}
	return ch, cancel
}

// Publish delivers msg to all current subscribers of topic. Slow subscribers
// whose buffers are full are skipped rather than blocking the publisher.
func (h *Hub) Publish(topic string, msg []byte) {
	h.mu.RLock()
	defer h.mu.RUnlock()
	for ch := range h.subs[topic] {
		select {
		case ch <- msg:
		default:
		}
	}
}
