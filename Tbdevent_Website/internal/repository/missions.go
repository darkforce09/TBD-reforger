package repository

import (
	"context"
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
	"errors"
	"fmt"
	"regexp"
	"time"

	"github.com/google/uuid"
	"github.com/jackc/pgx/v5"
	"github.com/jackc/pgx/v5/pgconn"

	"github.com/tbdevent/website/internal/models"
)

var (
	ErrLinkCodeInvalid  = errors.New("invalid or expired link code")
	ErrLinkCodeUsed     = errors.New("link code already consumed")
	ErrAlreadyLinked    = errors.New("game identity already linked")
	ErrMissionExists    = errors.New("mission id already published")
)

var linkCodePattern = regexp.MustCompile(`^[0-9]{6}$`)

func (r *Repository) PublishMission(ctx context.Context, raw []byte, authorID *uuid.UUID) (*models.Mission, error) {
	var doc struct {
		SchemaVersion string `json:"schemaVersion"`
		Meta          struct {
			ID   string `json:"id"`
			Name string `json:"name"`
		} `json:"meta"`
	}
	if err := json.Unmarshal(raw, &doc); err != nil {
		return nil, fmt.Errorf("parse mission: %w", err)
	}

	hash := sha256.Sum256(raw)
	contentHash := hex.EncodeToString(hash[:])

	var m models.Mission
	err := r.pool.QueryRow(ctx, `
		INSERT INTO missions (id, name, schema_version, content, content_hash, author_user_id)
		VALUES ($1, $2, $3, $4, $5, $6)
		RETURNING id, name, schema_version, content, content_hash, author_user_id, published_at, created_at
	`, doc.Meta.ID, doc.Meta.Name, doc.SchemaVersion, raw, contentHash, authorID).Scan(
		&m.ID, &m.Name, &m.SchemaVersion, &m.Content, &m.ContentHash,
		&m.AuthorUserID, &m.PublishedAt, &m.CreatedAt,
	)
	if err != nil {
		if isUniqueViolation(err) {
			return nil, ErrMissionExists
		}
		return nil, fmt.Errorf("publish mission: %w", err)
	}
	return &m, nil
}

// ListMissions returns lightweight summaries of all published missions for the
// in-game admin mission browser. Terrain and slot count are extracted from the
// mission content JSONB so the game can route a selection to the right scenario.
func (r *Repository) ListMissions(ctx context.Context) ([]models.MissionSummary, error) {
	rows, err := r.pool.Query(ctx, `
		SELECT id, name, schema_version,
		       COALESCE(content->'meta'->>'terrain', '') AS terrain,
		       CASE WHEN jsonb_typeof(content->'slots') = 'array'
		            THEN jsonb_array_length(content->'slots') ELSE 0 END AS slot_count,
		       published_at
		FROM missions
		ORDER BY published_at DESC
	`)
	if err != nil {
		return nil, fmt.Errorf("list missions: %w", err)
	}
	defer rows.Close()

	out := make([]models.MissionSummary, 0)
	for rows.Next() {
		var m models.MissionSummary
		if err := rows.Scan(&m.ID, &m.Name, &m.SchemaVersion, &m.Terrain, &m.SlotCount, &m.PublishedAt); err != nil {
			return nil, fmt.Errorf("scan mission summary: %w", err)
		}
		out = append(out, m)
	}
	return out, rows.Err()
}

func (r *Repository) GetMissionCompiled(ctx context.Context, id string) ([]byte, error) {
	var content []byte
	err := r.pool.QueryRow(ctx, `SELECT content FROM missions WHERE id = $1`, id).Scan(&content)
	if err != nil {
		if errors.Is(err, pgx.ErrNoRows) {
			return nil, ErrNotFound
		}
		return nil, fmt.Errorf("get mission: %w", err)
	}
	return content, nil
}

func (r *Repository) SetEventMission(ctx context.Context, eventID uuid.UUID, missionID string) error {
	tag, err := r.pool.Exec(ctx, `
		UPDATE events SET mission_id = $2, updated_at = NOW() WHERE id = $1
	`, eventID, missionID)
	if err != nil {
		return fmt.Errorf("set event mission: %w", err)
	}
	if tag.RowsAffected() == 0 {
		return ErrNotFound
	}
	return nil
}

func (r *Repository) RegisterLinkCode(ctx context.Context, input models.RegisterLinkCodeInput) error {
	if !linkCodePattern.MatchString(input.Code) || input.IdentityID == "" {
		return ErrLinkCodeInvalid
	}

	platform := input.Platform
	if platform == "" {
		platform = "pc"
	}

	expires := time.Now().Add(15 * time.Minute)
	_, err := r.pool.Exec(ctx, `
		INSERT INTO link_codes (code, identity_id, event_id, expires_at)
		VALUES ($1, $2, $3, $4)
		ON CONFLICT (code) DO UPDATE SET
			identity_id = EXCLUDED.identity_id,
			event_id = EXCLUDED.event_id,
			created_at = NOW(),
			expires_at = EXCLUDED.expires_at,
			consumed_at = NULL
	`, input.Code, input.IdentityID, input.EventID, expires)
	if err != nil {
		return fmt.Errorf("register link code: %w", err)
	}
	return nil
}

func (r *Repository) ConsumeLinkCode(ctx context.Context, userID uuid.UUID, code string) (*models.GameIdentity, error) {
	if !linkCodePattern.MatchString(code) {
		return nil, ErrLinkCodeInvalid
	}

	tx, err := r.pool.Begin(ctx)
	if err != nil {
		return nil, err
	}
	defer tx.Rollback(ctx)

	var identityID string
	var expiresAt time.Time
	var consumedAt *time.Time
	err = tx.QueryRow(ctx, `
		SELECT identity_id, expires_at, consumed_at FROM link_codes WHERE code = $1 FOR UPDATE
	`, code).Scan(&identityID, &expiresAt, &consumedAt)
	if err != nil {
		if errors.Is(err, pgx.ErrNoRows) {
			return nil, ErrLinkCodeInvalid
		}
		return nil, fmt.Errorf("lookup link code: %w", err)
	}

	if consumedAt != nil {
		return nil, ErrLinkCodeUsed
	}
	if time.Now().After(expiresAt) {
		return nil, ErrLinkCodeInvalid
	}

	var existing uuid.UUID
	err = tx.QueryRow(ctx, `SELECT user_id FROM game_identities WHERE user_id = $1`, userID).Scan(&existing)
	if err == nil {
		return nil, ErrAlreadyLinked
	}
	if !errors.Is(err, pgx.ErrNoRows) {
		return nil, fmt.Errorf("check existing link: %w", err)
	}

	var gi models.GameIdentity
	err = tx.QueryRow(ctx, `
		INSERT INTO game_identities (user_id, identity_id)
		VALUES ($1, $2)
		RETURNING id, user_id, identity_id, platform, linked_at
	`, userID, identityID).Scan(&gi.ID, &gi.UserID, &gi.IdentityID, &gi.Platform, &gi.LinkedAt)
	if err != nil {
		if isUniqueViolation(err) {
			return nil, ErrAlreadyLinked
		}
		return nil, fmt.Errorf("insert identity: %w", err)
	}

	now := time.Now()
	_, err = tx.Exec(ctx, `UPDATE link_codes SET consumed_at = $2 WHERE code = $1`, code, now)
	if err != nil {
		return nil, fmt.Errorf("consume link code: %w", err)
	}

	if err := tx.Commit(ctx); err != nil {
		return nil, err
	}
	return &gi, nil
}

func (r *Repository) GetGameIdentityByUser(ctx context.Context, userID uuid.UUID) (*models.GameIdentity, error) {
	var gi models.GameIdentity
	err := r.pool.QueryRow(ctx, `
		SELECT id, user_id, identity_id, platform, linked_at
		FROM game_identities WHERE user_id = $1
	`, userID).Scan(&gi.ID, &gi.UserID, &gi.IdentityID, &gi.Platform, &gi.LinkedAt)
	if err != nil {
		if errors.Is(err, pgx.ErrNoRows) {
			return nil, ErrNotFound
		}
		return nil, fmt.Errorf("get game identity: %w", err)
	}
	return &gi, nil
}

func (r *Repository) AssignEventSlot(ctx context.Context, eventID uuid.UUID, slotID string, userID *uuid.UUID, assignedBy uuid.UUID) (*models.EventSlotAssignment, error) {
	var a models.EventSlotAssignment
	err := r.pool.QueryRow(ctx, `
		INSERT INTO event_slot_assignments (event_id, slot_id, user_id, assigned_by)
		VALUES ($1, $2, $3, $4)
		ON CONFLICT (event_id, slot_id) DO UPDATE SET
			user_id = EXCLUDED.user_id,
			assigned_by = EXCLUDED.assigned_by,
			assigned_at = NOW()
		RETURNING id, event_id, slot_id, user_id, assigned_by, assigned_at
	`, eventID, slotID, userID, assignedBy).Scan(
		&a.ID, &a.EventID, &a.SlotID, &a.UserID, &a.AssignedBy, &a.AssignedAt,
	)
	if err != nil {
		return nil, fmt.Errorf("assign slot: %w", err)
	}
	return &a, nil
}

func (r *Repository) ListEventSlotAssignments(ctx context.Context, eventID uuid.UUID) ([]models.EventSlotAssignment, error) {
	rows, err := r.pool.Query(ctx, `
		SELECT esa.id, esa.event_id, esa.slot_id, esa.user_id, esa.assigned_by, esa.assigned_at,
		       u.username, u.avatar_url, u.discord_id
		FROM event_slot_assignments esa
		LEFT JOIN users u ON u.id = esa.user_id
		WHERE esa.event_id = $1
		ORDER BY esa.slot_id ASC
	`, eventID)
	if err != nil {
		return nil, fmt.Errorf("list slot assignments: %w", err)
	}
	defer rows.Close()

	var out []models.EventSlotAssignment
	for rows.Next() {
		var a models.EventSlotAssignment
		var user models.User
		var username, avatar *string
		var discordID *string
		if err := rows.Scan(
			&a.ID, &a.EventID, &a.SlotID, &a.UserID, &a.AssignedBy, &a.AssignedAt,
			&username, &avatar, &discordID,
		); err != nil {
			return nil, fmt.Errorf("scan slot assignment: %w", err)
		}
		if username != nil {
			user.Username = *username
			if avatar != nil {
				user.AvatarURL = *avatar
			}
			if discordID != nil {
				user.DiscordID = *discordID
			}
			if a.UserID != nil {
				user.ID = *a.UserID
			}
			a.User = &user
		}
		out = append(out, a)
	}
	return out, rows.Err()
}

func (r *Repository) GameRosterForEvent(ctx context.Context, eventID uuid.UUID) (*models.GameRosterResponse, error) {
	event, err := r.GetEventByID(ctx, eventID)
	if err != nil {
		return nil, err
	}

	rows, err := r.pool.Query(ctx, `
		SELECT gi.identity_id, esa.slot_id
		FROM event_slot_assignments esa
		JOIN game_identities gi ON gi.user_id = esa.user_id
		WHERE esa.event_id = $1 AND esa.user_id IS NOT NULL
	`, eventID)
	if err != nil {
		return nil, fmt.Errorf("game roster query: %w", err)
	}
	defer rows.Close()

	assignments := make(map[string]string)
	for rows.Next() {
		var identityID, slotID string
		if err := rows.Scan(&identityID, &slotID); err != nil {
			return nil, err
		}
		assignments[identityID] = slotID
	}
	if err := rows.Err(); err != nil {
		return nil, err
	}

	resp := &models.GameRosterResponse{
		EventID:     eventID,
		Assignments: assignments,
	}
	if event.MissionID != nil {
		resp.MissionID = *event.MissionID
	}
	return resp, nil
}

func isUniqueViolation(err error) bool {
	var pgErr *pgconn.PgError
	return errors.As(err, &pgErr) && pgErr.Code == "23505"
}
