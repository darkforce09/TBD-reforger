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

func (r *Repository) getUserRegistrationForEvent(ctx context.Context, eventID, userID uuid.UUID) (*models.Registration, error) {
	var reg models.Registration
	err := r.pool.QueryRow(ctx, `
		SELECT id, event_id, user_id, status, signed_up_at, cancelled_at
		FROM event_registrations
		WHERE event_id = $1 AND user_id = $2 AND status != 'cancelled'
	`, eventID, userID).Scan(
		&reg.ID, &reg.EventID, &reg.UserID, &reg.Status, &reg.SignedUpAt, &reg.CancelledAt,
	)
	if err != nil {
		if errors.Is(err, pgx.ErrNoRows) {
			return nil, ErrNotFound
		}
		return nil, fmt.Errorf("get registration: %w", err)
	}
	return &reg, nil
}

func (r *Repository) countRegistered(ctx context.Context, eventID uuid.UUID) (int, error) {
	var count int
	err := r.pool.QueryRow(ctx, `
		SELECT COUNT(*) FROM event_registrations
		WHERE event_id = $1 AND status = 'registered'
	`, eventID).Scan(&count)
	return count, err
}

func (r *Repository) RegisterForEvent(ctx context.Context, eventSlug string, userID uuid.UUID) (*models.Registration, error) {
	var event models.Event
	err := r.pool.QueryRow(ctx, `
		SELECT id, max_players, signups_open, status, published
		FROM events WHERE slug = $1
	`, eventSlug).Scan(&event.ID, &event.MaxPlayers, &event.SignupsOpen, &event.Status, &event.Published)
	if err != nil {
		if errors.Is(err, pgx.ErrNoRows) {
			return nil, ErrNotFound
		}
		return nil, fmt.Errorf("get event for register: %w", err)
	}

	if !event.Published || event.Status != "published" || !event.SignupsOpen {
		return nil, ErrSignupsClosed
	}

	existing, err := r.getUserRegistrationForEvent(ctx, event.ID, userID)
	if err != nil && !errors.Is(err, ErrNotFound) {
		return nil, err
	}
	if existing != nil {
		return existing, nil
	}

	// Check for cancelled registration to reactivate
	var cancelledID uuid.UUID
	err = r.pool.QueryRow(ctx, `
		SELECT id FROM event_registrations
		WHERE event_id = $1 AND user_id = $2 AND status = 'cancelled'
	`, event.ID, userID).Scan(&cancelledID)

	status := "registered"
	if event.MaxPlayers != nil {
		count, err := r.countRegistered(ctx, event.ID)
		if err != nil {
			return nil, err
		}
		if count >= *event.MaxPlayers {
			status = "waitlist"
		}
	}

	if err == nil {
		var reg models.Registration
		err = r.pool.QueryRow(ctx, `
			UPDATE event_registrations
			SET status = $2, signed_up_at = NOW(), cancelled_at = NULL
			WHERE id = $1
			RETURNING id, event_id, user_id, status, signed_up_at, cancelled_at
		`, cancelledID, status).Scan(
			&reg.ID, &reg.EventID, &reg.UserID, &reg.Status, &reg.SignedUpAt, &reg.CancelledAt,
		)
		if err != nil {
			return nil, fmt.Errorf("reactivate registration: %w", err)
		}
		return &reg, nil
	}

	var reg models.Registration
	err = r.pool.QueryRow(ctx, `
		INSERT INTO event_registrations (event_id, user_id, status)
		VALUES ($1, $2, $3)
		RETURNING id, event_id, user_id, status, signed_up_at, cancelled_at
	`, event.ID, userID, status).Scan(
		&reg.ID, &reg.EventID, &reg.UserID, &reg.Status, &reg.SignedUpAt, &reg.CancelledAt,
	)
	if err != nil {
		return nil, fmt.Errorf("create registration: %w", err)
	}
	return &reg, nil
}

func (r *Repository) CancelRegistration(ctx context.Context, eventSlug string, userID uuid.UUID) error {
	var eventID uuid.UUID
	err := r.pool.QueryRow(ctx, `SELECT id FROM events WHERE slug = $1`, eventSlug).Scan(&eventID)
	if err != nil {
		if errors.Is(err, pgx.ErrNoRows) {
			return ErrNotFound
		}
		return err
	}

	tag, err := r.pool.Exec(ctx, `
		UPDATE event_registrations
		SET status = 'cancelled', cancelled_at = NOW()
		WHERE event_id = $1 AND user_id = $2 AND status != 'cancelled'
	`, eventID, userID)
	if err != nil {
		return fmt.Errorf("cancel registration: %w", err)
	}
	if tag.RowsAffected() == 0 {
		return ErrNotFound
	}
	return nil
}

func (r *Repository) ListUserRegistrations(ctx context.Context, userID uuid.UUID) ([]models.Registration, error) {
	rows, err := r.pool.Query(ctx, `
		SELECT er.id, er.event_id, er.user_id, er.status, er.signed_up_at, er.cancelled_at,
		       e.title, e.slug, e.map_name, e.starts_at, e.ends_at, e.status,
		       e.max_players, e.signups_open,
		       COALESCE((SELECT COUNT(*) FROM event_registrations er2
		                 WHERE er2.event_id = e.id AND er2.status = 'registered'), 0)
		FROM event_registrations er
		JOIN events e ON e.id = er.event_id
		WHERE er.user_id = $1 AND er.status != 'cancelled'
		  AND e.starts_at >= NOW()
		ORDER BY e.starts_at ASC
	`, userID)
	if err != nil {
		return nil, fmt.Errorf("list user registrations: %w", err)
	}
	defer rows.Close()

	var regs []models.Registration
	for rows.Next() {
		var reg models.Registration
		var ev models.EventSummary
		if err := rows.Scan(
			&reg.ID, &reg.EventID, &reg.UserID, &reg.Status, &reg.SignedUpAt, &reg.CancelledAt,
			&ev.Title, &ev.Slug, &ev.MapName, &ev.StartsAt, &ev.EndsAt, &ev.Status,
			&ev.MaxPlayers, &ev.SignupsOpen, &ev.Registered,
		); err != nil {
			return nil, fmt.Errorf("scan registration: %w", err)
		}
		ev.ID = reg.EventID
		reg.Event = &ev
		regs = append(regs, reg)
	}
	return regs, rows.Err()
}

func (r *Repository) ListEventRegistrations(ctx context.Context, eventID uuid.UUID) ([]models.Registration, error) {
	rows, err := r.pool.Query(ctx, `
		SELECT er.id, er.event_id, er.user_id, er.status, er.signed_up_at, er.cancelled_at,
		       u.username, u.avatar_url, u.discord_id
		FROM event_registrations er
		JOIN users u ON u.id = er.user_id
		WHERE er.event_id = $1 AND er.status != 'cancelled'
		ORDER BY
			CASE er.status WHEN 'registered' THEN 0 WHEN 'waitlist' THEN 1 ELSE 2 END,
			er.signed_up_at ASC
	`, eventID)
	if err != nil {
		return nil, fmt.Errorf("list event registrations: %w", err)
	}
	defer rows.Close()

	var regs []models.Registration
	for rows.Next() {
		var reg models.Registration
		var user models.User
		if err := rows.Scan(
			&reg.ID, &reg.EventID, &reg.UserID, &reg.Status, &reg.SignedUpAt, &reg.CancelledAt,
			&user.Username, &user.AvatarURL, &user.DiscordID,
		); err != nil {
			return nil, fmt.Errorf("scan registration: %w", err)
		}
		user.ID = reg.UserID
		reg.User = &user
		regs = append(regs, reg)
	}
	return regs, rows.Err()
}

func (r *Repository) ListPublicRoster(ctx context.Context, eventID uuid.UUID) ([]models.Registration, error) {
	rows, err := r.pool.Query(ctx, `
		SELECT er.id, er.event_id, er.user_id, er.status, er.signed_up_at,
		       u.username, u.avatar_url
		FROM event_registrations er
		JOIN users u ON u.id = er.user_id
		WHERE er.event_id = $1 AND er.status = 'registered'
		ORDER BY er.signed_up_at ASC
	`, eventID)
	if err != nil {
		return nil, fmt.Errorf("list roster: %w", err)
	}
	defer rows.Close()

	var regs []models.Registration
	for rows.Next() {
		var reg models.Registration
		var user models.User
		if err := rows.Scan(
			&reg.ID, &reg.EventID, &reg.UserID, &reg.Status, &reg.SignedUpAt,
			&user.Username, &user.AvatarURL,
		); err != nil {
			return nil, fmt.Errorf("scan roster: %w", err)
		}
		user.ID = reg.UserID
		reg.User = &user
		regs = append(regs, reg)
	}
	return regs, rows.Err()
}

func (r *Repository) UpdateRegistrationStatus(ctx context.Context, id uuid.UUID, status string) (*models.Registration, error) {
	var reg models.Registration
	cancelledAt := (*time.Time)(nil)
	if status == "cancelled" {
		now := time.Now()
		cancelledAt = &now
	}
	err := r.pool.QueryRow(ctx, `
		UPDATE event_registrations SET status = $2, cancelled_at = $3
		WHERE id = $1
		RETURNING id, event_id, user_id, status, signed_up_at, cancelled_at
	`, id, status, cancelledAt).Scan(
		&reg.ID, &reg.EventID, &reg.UserID, &reg.Status, &reg.SignedUpAt, &reg.CancelledAt,
	)
	if err != nil {
		if errors.Is(err, pgx.ErrNoRows) {
			return nil, ErrNotFound
		}
		return nil, fmt.Errorf("update registration: %w", err)
	}
	return &reg, nil
}

func (r *Repository) DeleteRegistration(ctx context.Context, id uuid.UUID) error {
	tag, err := r.pool.Exec(ctx, `DELETE FROM event_registrations WHERE id = $1`, id)
	if err != nil {
		return fmt.Errorf("delete registration: %w", err)
	}
	if tag.RowsAffected() == 0 {
		return ErrNotFound
	}
	return nil
}
