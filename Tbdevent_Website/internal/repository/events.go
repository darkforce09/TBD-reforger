package repository

import (
	"context"
	"errors"
	"fmt"
	"time"

	"github.com/google/uuid"
	"github.com/jackc/pgx/v5"

	"github.com/tbdevent/website/internal/models"
)

func (r *Repository) ListEvents(ctx context.Context, upcoming, past bool) ([]models.EventSummary, error) {
	query := `
		SELECT e.id, e.title, e.slug, e.map_name, e.starts_at, e.ends_at, e.status,
		       e.max_players, e.signups_open,
		       COALESCE((SELECT COUNT(*) FROM event_registrations er
		                 WHERE er.event_id = e.id AND er.status = 'registered'), 0)
		FROM events e
		WHERE e.published = TRUE
	`
	if upcoming {
		query += ` AND e.starts_at >= NOW() AND e.status NOT IN ('completed', 'cancelled')`
	}
	if past {
		query += ` AND (e.starts_at < NOW() OR e.status IN ('completed', 'cancelled'))`
	}

	query += ` ORDER BY e.starts_at ASC`

	rows, err := r.pool.Query(ctx, query)
	if err != nil {
		return nil, fmt.Errorf("list events: %w", err)
	}
	defer rows.Close()

	var events []models.EventSummary
	for rows.Next() {
		var e models.EventSummary
		if err := rows.Scan(&e.ID, &e.Title, &e.Slug, &e.MapName, &e.StartsAt, &e.EndsAt,
			&e.Status, &e.MaxPlayers, &e.SignupsOpen, &e.Registered); err != nil {
			return nil, fmt.Errorf("scan event: %w", err)
		}
		events = append(events, e)
	}
	return events, rows.Err()
}

func (r *Repository) GetEventBySlug(ctx context.Context, slug string, userID *uuid.UUID) (*models.Event, error) {
	var e models.Event
	err := r.pool.QueryRow(ctx, `
		SELECT id, title, slug, description, map_name, starts_at, ends_at, status,
		       max_players, signups_open, published, mission_id, created_at, updated_at
		FROM events WHERE slug = $1 AND published = TRUE
	`, slug).Scan(
		&e.ID, &e.Title, &e.Slug, &e.Description, &e.MapName, &e.StartsAt, &e.EndsAt,
		&e.Status, &e.MaxPlayers, &e.SignupsOpen, &e.Published, &e.MissionID, &e.CreatedAt, &e.UpdatedAt,
	)
	if err != nil {
		if errors.Is(err, pgx.ErrNoRows) {
			return nil, ErrNotFound
		}
		return nil, fmt.Errorf("get event: %w", err)
	}

	if err := r.attachEventCounts(ctx, &e); err != nil {
		return nil, err
	}

	if userID != nil {
		reg, err := r.getUserRegistrationForEvent(ctx, e.ID, *userID)
		if err != nil && !errors.Is(err, ErrNotFound) {
			return nil, err
		}
		if reg != nil {
			e.UserReg = reg
		}
	}

	return &e, nil
}

func (r *Repository) attachEventCounts(ctx context.Context, e *models.Event) error {
	err := r.pool.QueryRow(ctx, `
		SELECT
			COALESCE(SUM(CASE WHEN status = 'registered' THEN 1 ELSE 0 END), 0),
			COALESCE(SUM(CASE WHEN status = 'waitlist' THEN 1 ELSE 0 END), 0)
		FROM event_registrations WHERE event_id = $1
	`, e.ID).Scan(&e.Registered, &e.Waitlisted)
	if err != nil {
		return fmt.Errorf("count registrations: %w", err)
	}
	return nil
}

func (r *Repository) GetNextEvent(ctx context.Context) (*models.EventSummary, error) {
	var e models.EventSummary
	err := r.pool.QueryRow(ctx, `
		SELECT e.id, e.title, e.slug, e.map_name, e.starts_at, e.ends_at, e.status,
		       e.max_players, e.signups_open,
		       COALESCE((SELECT COUNT(*) FROM event_registrations er
		                 WHERE er.event_id = e.id AND er.status = 'registered'), 0)
		FROM events e
		WHERE e.published = TRUE AND e.starts_at >= NOW()
		  AND e.status NOT IN ('completed', 'cancelled')
		ORDER BY e.starts_at ASC
		LIMIT 1
	`).Scan(&e.ID, &e.Title, &e.Slug, &e.MapName, &e.StartsAt, &e.EndsAt,
		&e.Status, &e.MaxPlayers, &e.SignupsOpen, &e.Registered)
	if err != nil {
		if errors.Is(err, pgx.ErrNoRows) {
			return nil, ErrNotFound
		}
		return nil, fmt.Errorf("get next event: %w", err)
	}
	return &e, nil
}

func (r *Repository) ListAllEvents(ctx context.Context) ([]models.Event, error) {
	rows, err := r.pool.Query(ctx, `
		SELECT id, title, slug, description, map_name, starts_at, ends_at, status,
		       max_players, signups_open, published, created_at, updated_at
		FROM events ORDER BY starts_at DESC
	`)
	if err != nil {
		return nil, fmt.Errorf("list all events: %w", err)
	}
	defer rows.Close()

	var events []models.Event
	for rows.Next() {
		var e models.Event
		if err := rows.Scan(&e.ID, &e.Title, &e.Slug, &e.Description, &e.MapName,
			&e.StartsAt, &e.EndsAt, &e.Status, &e.MaxPlayers, &e.SignupsOpen,
			&e.Published, &e.CreatedAt, &e.UpdatedAt); err != nil {
			return nil, fmt.Errorf("scan event: %w", err)
		}
		_ = r.attachEventCounts(ctx, &e)
		events = append(events, e)
	}
	return events, rows.Err()
}

func (r *Repository) GetEventByID(ctx context.Context, id uuid.UUID) (*models.Event, error) {
	var e models.Event
	err := r.pool.QueryRow(ctx, `
		SELECT id, title, slug, description, map_name, starts_at, ends_at, status,
		       max_players, signups_open, published, mission_id, created_at, updated_at
		FROM events WHERE id = $1
	`, id).Scan(
		&e.ID, &e.Title, &e.Slug, &e.Description, &e.MapName, &e.StartsAt, &e.EndsAt,
		&e.Status, &e.MaxPlayers, &e.SignupsOpen, &e.Published, &e.MissionID, &e.CreatedAt, &e.UpdatedAt,
	)
	if err != nil {
		if errors.Is(err, pgx.ErrNoRows) {
			return nil, ErrNotFound
		}
		return nil, fmt.Errorf("get event by id: %w", err)
	}
	_ = r.attachEventCounts(ctx, &e)
	return &e, nil
}

func (r *Repository) CreateEvent(ctx context.Context, input models.CreateEventInput) (*models.Event, error) {
	var e models.Event
	err := r.pool.QueryRow(ctx, `
		INSERT INTO events (title, slug, description, map_name, starts_at, ends_at, status,
		                    max_players, signups_open, published, mission_id, updated_at)
		VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, NOW())
		RETURNING id, title, slug, description, map_name, starts_at, ends_at, status,
		          max_players, signups_open, published, mission_id, created_at, updated_at
	`, input.Title, input.Slug, input.Description, input.MapName, input.StartsAt,
		input.EndsAt, input.Status, input.MaxPlayers, input.SignupsOpen, input.Published, input.MissionID,
	).Scan(
		&e.ID, &e.Title, &e.Slug, &e.Description, &e.MapName, &e.StartsAt, &e.EndsAt,
		&e.Status, &e.MaxPlayers, &e.SignupsOpen, &e.Published, &e.MissionID, &e.CreatedAt, &e.UpdatedAt,
	)
	if err != nil {
		return nil, fmt.Errorf("create event: %w", err)
	}
	return &e, nil
}

func (r *Repository) UpdateEvent(ctx context.Context, id uuid.UUID, input models.UpdateEventInput) (*models.Event, error) {
	current, err := r.GetEventByID(ctx, id)
	if err != nil {
		return nil, err
	}

	title := current.Title
	slug := current.Slug
	description := current.Description
	mapName := current.MapName
	startsAt := current.StartsAt
	endsAt := current.EndsAt
	status := current.Status
	maxPlayers := current.MaxPlayers
	signupsOpen := current.SignupsOpen
	published := current.Published
	missionID := current.MissionID

	if input.Title != nil { title = *input.Title }
	if input.Slug != nil { slug = *input.Slug }
	if input.Description != nil { description = *input.Description }
	if input.MapName != nil { mapName = *input.MapName }
	if input.StartsAt != nil { startsAt = *input.StartsAt }
	if input.EndsAt != nil { endsAt = input.EndsAt }
	if input.Status != nil { status = *input.Status }
	if input.MaxPlayers != nil { maxPlayers = input.MaxPlayers }
	if input.SignupsOpen != nil { signupsOpen = *input.SignupsOpen }
	if input.Published != nil { published = *input.Published }
	if input.MissionID != nil { missionID = input.MissionID }

	var e models.Event
	err = r.pool.QueryRow(ctx, `
		UPDATE events SET title=$2, slug=$3, description=$4, map_name=$5, starts_at=$6,
		                  ends_at=$7, status=$8, max_players=$9, signups_open=$10,
		                  published=$11, mission_id=$12, updated_at=NOW()
		WHERE id=$1
		RETURNING id, title, slug, description, map_name, starts_at, ends_at, status,
		          max_players, signups_open, published, mission_id, created_at, updated_at
	`, id, title, slug, description, mapName, startsAt, endsAt, status,
		maxPlayers, signupsOpen, published, missionID,
	).Scan(
		&e.ID, &e.Title, &e.Slug, &e.Description, &e.MapName, &e.StartsAt, &e.EndsAt,
		&e.Status, &e.MaxPlayers, &e.SignupsOpen, &e.Published, &e.MissionID, &e.CreatedAt, &e.UpdatedAt,
	)
	if err != nil {
		return nil, fmt.Errorf("update event: %w", err)
	}
	return &e, nil
}

func (r *Repository) DeleteEvent(ctx context.Context, id uuid.UUID) error {
	tag, err := r.pool.Exec(ctx, `DELETE FROM events WHERE id = $1`, id)
	if err != nil {
		return fmt.Errorf("delete event: %w", err)
	}
	if tag.RowsAffected() == 0 {
		return ErrNotFound
	}
	return nil
}

func (r *Repository) ListAnnouncements(ctx context.Context, limit int) ([]models.Announcement, error) {
	if limit <= 0 {
		limit = 50
	}
	rows, err := r.pool.Query(ctx, `
		SELECT id, title, body, pinned, published, published_at, created_at, updated_at
		FROM announcements
		WHERE published = TRUE
		ORDER BY pinned DESC, published_at DESC NULLS LAST, created_at DESC
		LIMIT $1
	`, limit)
	if err != nil {
		return nil, fmt.Errorf("list announcements: %w", err)
	}
	defer rows.Close()

	var items []models.Announcement
	for rows.Next() {
		var a models.Announcement
		if err := rows.Scan(&a.ID, &a.Title, &a.Body, &a.Pinned, &a.Published,
			&a.PublishedAt, &a.CreatedAt, &a.UpdatedAt); err != nil {
			return nil, fmt.Errorf("scan announcement: %w", err)
		}
		items = append(items, a)
	}
	return items, rows.Err()
}

func (r *Repository) ListAllAnnouncements(ctx context.Context) ([]models.Announcement, error) {
	rows, err := r.pool.Query(ctx, `
		SELECT id, title, body, pinned, published, published_at, created_at, updated_at
		FROM announcements ORDER BY pinned DESC, created_at DESC
	`)
	if err != nil {
		return nil, fmt.Errorf("list all announcements: %w", err)
	}
	defer rows.Close()

	var items []models.Announcement
	for rows.Next() {
		var a models.Announcement
		if err := rows.Scan(&a.ID, &a.Title, &a.Body, &a.Pinned, &a.Published,
			&a.PublishedAt, &a.CreatedAt, &a.UpdatedAt); err != nil {
			return nil, fmt.Errorf("scan announcement: %w", err)
		}
		items = append(items, a)
	}
	return items, rows.Err()
}

func (r *Repository) CreateAnnouncement(ctx context.Context, input models.CreateAnnouncementInput) (*models.Announcement, error) {
	pubAt := input.PublishedAt
	if pubAt == nil && input.Published {
		now := time.Now()
		pubAt = &now
	}
	var a models.Announcement
	err := r.pool.QueryRow(ctx, `
		INSERT INTO announcements (title, body, pinned, published, published_at, updated_at)
		VALUES ($1, $2, $3, $4, $5, NOW())
		RETURNING id, title, body, pinned, published, published_at, created_at, updated_at
	`, input.Title, input.Body, input.Pinned, input.Published, pubAt,
	).Scan(&a.ID, &a.Title, &a.Body, &a.Pinned, &a.Published, &a.PublishedAt, &a.CreatedAt, &a.UpdatedAt)
	if err != nil {
		return nil, fmt.Errorf("create announcement: %w", err)
	}
	return &a, nil
}

func (r *Repository) UpdateAnnouncement(ctx context.Context, id uuid.UUID, input models.UpdateAnnouncementInput) (*models.Announcement, error) {
	rows, err := r.pool.Query(ctx, `
		SELECT title, body, pinned, published, published_at FROM announcements WHERE id = $1
	`, id)
	if err != nil {
		return nil, fmt.Errorf("get announcement: %w", err)
	}
	defer rows.Close()
	if !rows.Next() {
		return nil, ErrNotFound
	}
	var title, body string
	var pinned, published bool
	var publishedAt *time.Time
	if err := rows.Scan(&title, &body, &pinned, &published, &publishedAt); err != nil {
		return nil, err
	}

	if input.Title != nil { title = *input.Title }
	if input.Body != nil { body = *input.Body }
	if input.Pinned != nil { pinned = *input.Pinned }
	if input.Published != nil { published = *input.Published }
	if input.PublishedAt != nil { publishedAt = input.PublishedAt }

	var a models.Announcement
	err = r.pool.QueryRow(ctx, `
		UPDATE announcements SET title=$2, body=$3, pinned=$4, published=$5,
		                       published_at=$6, updated_at=NOW()
		WHERE id=$1
		RETURNING id, title, body, pinned, published, published_at, created_at, updated_at
	`, id, title, body, pinned, published, publishedAt,
	).Scan(&a.ID, &a.Title, &a.Body, &a.Pinned, &a.Published, &a.PublishedAt, &a.CreatedAt, &a.UpdatedAt)
	if err != nil {
		return nil, fmt.Errorf("update announcement: %w", err)
	}
	return &a, nil
}

func (r *Repository) DeleteAnnouncement(ctx context.Context, id uuid.UUID) error {
	tag, err := r.pool.Exec(ctx, `DELETE FROM announcements WHERE id = $1`, id)
	if err != nil {
		return fmt.Errorf("delete announcement: %w", err)
	}
	if tag.RowsAffected() == 0 {
		return ErrNotFound
	}
	return nil
}
