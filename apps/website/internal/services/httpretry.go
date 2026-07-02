package services

import (
	"net/http"
	"strconv"
	"time"
)

// Discord 429 retry policy (T-130.2 F3-01): bounded attempts, honoring the
// Retry-After header (Discord sends fractional seconds). The wait is clamped so a
// hostile/global rate-limit response cannot park a request past the HTTP client's
// own 10s timeout.
const (
	max429Attempts    = 3
	default429Backoff = time.Second
	max429Backoff     = 5 * time.Second
)

// parseRetryAfter converts a Retry-After header value (seconds, possibly
// fractional) into a bounded wait, falling back to default429Backoff when the
// header is absent or malformed.
func parseRetryAfter(v string) time.Duration {
	secs, err := strconv.ParseFloat(v, 64)
	if err != nil || secs < 0 {
		return default429Backoff
	}
	d := time.Duration(secs * float64(time.Second))
	if d > max429Backoff {
		return max429Backoff
	}
	return d
}

// doWithRetryOn429 executes req via client, retrying up to max429Attempts times
// while the response is 429 Too Many Requests. Replayable bodies (req.GetBody —
// set by net/http for strings/bytes readers) are rewound between attempts. The
// final 429 response is returned to the caller, whose normal non-2xx handling
// surfaces it; waits abort early if the request context ends.
func doWithRetryOn429(client *http.Client, req *http.Request) (*http.Response, error) {
	for attempt := 1; ; attempt++ {
		if attempt > 1 && req.GetBody != nil {
			body, err := req.GetBody()
			if err != nil {
				return nil, err
			}
			req.Body = body
		}
		resp, err := client.Do(req)
		if err != nil {
			return nil, err
		}
		if resp.StatusCode != http.StatusTooManyRequests || attempt == max429Attempts {
			return resp, nil
		}
		wait := parseRetryAfter(resp.Header.Get("Retry-After"))
		//nolint:errcheck // best-effort: the 429 body is discarded before the retry.
		resp.Body.Close()
		select {
		case <-time.After(wait):
		case <-req.Context().Done():
			return nil, req.Context().Err()
		}
	}
}
