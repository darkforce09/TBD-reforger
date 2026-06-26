package realtime

import (
	"testing"
	"time"
)

func TestHubPublishDelivers(t *testing.T) {
	h := NewHub()
	ch, cancel := h.Subscribe("topic-a")
	defer cancel()

	h.Publish("topic-a", []byte("hello"))
	select {
	case msg := <-ch:
		if string(msg) != "hello" {
			t.Fatalf("got %q", msg)
		}
	case <-time.After(time.Second):
		t.Fatal("timed out waiting for message")
	}
}

func TestHubTopicIsolation(t *testing.T) {
	h := NewHub()
	ch, cancel := h.Subscribe("topic-a")
	defer cancel()

	h.Publish("topic-b", []byte("nope"))
	select {
	case msg := <-ch:
		t.Fatalf("unexpected cross-topic message %q", msg)
	case <-time.After(100 * time.Millisecond):
		// correct: no delivery across topics
	}
}

func TestHubUnsubscribeStopsDelivery(t *testing.T) {
	h := NewHub()
	ch, cancel := h.Subscribe("topic-a")
	cancel()
	// Publishing after cancel must not panic and the channel is closed.
	h.Publish("topic-a", []byte("x"))
	if _, open := <-ch; open {
		t.Fatal("expected channel closed after cancel")
	}
}
