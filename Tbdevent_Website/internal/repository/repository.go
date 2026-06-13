package repository

import (
	"context"
	"errors"
	"fmt"
	"time"

	"github.com/google/uuid"
	"github.com/jackc/pgx/v5"
	"github.com/jackc/pgx/v5/pgxpool"

	"github.com/tbdevent/website/internal/models"
)

var (
	ErrNotFound        = errors.New("not found")
	ErrSignupsClosed   = errors.New("signups are not open")
)

type Repository struct {
	pool *pgxpool.Pool
}

func New(pool *pgxpool.Pool) *Repository {
	return &Repository{pool: pool}
}

func (r *Repository) ListPublishedPages(ctx context.Context) ([]models.PageSummary, error) {
	rows, err := r.pool.Query(ctx, `
		SELECT slug, title
		FROM pages
		WHERE published = TRUE
		ORDER BY slug
	`)
	if err != nil {
		return nil, fmt.Errorf("list pages: %w", err)
	}
	defer rows.Close()

	var pages []models.PageSummary
	for rows.Next() {
		var p models.PageSummary
		if err := rows.Scan(&p.Slug, &p.Title); err != nil {
			return nil, fmt.Errorf("scan page: %w", err)
		}
		pages = append(pages, p)
	}

	return pages, rows.Err()
}

func (r *Repository) GetPageBySlug(ctx context.Context, slug string, includeUnpublished bool) (*models.Page, error) {
	var page models.Page
	var updatedAt *time.Time

	query := `
		SELECT slug, title, published, updated_at
		FROM pages
		WHERE slug = $1
	`
	if !includeUnpublished {
		query += ` AND published = TRUE`
	}

	err := r.pool.QueryRow(ctx, query, slug).Scan(&page.Slug, &page.Title, &page.Published, &updatedAt)
	if err != nil {
		if errors.Is(err, pgx.ErrNoRows) {
			return nil, ErrNotFound
		}
		return nil, fmt.Errorf("get page: %w", err)
	}
	page.UpdatedAt = updatedAt

	sections, err := r.listSectionsForSlug(ctx, slug)
	if err != nil {
		return nil, err
	}
	page.Sections = sections

	return &page, nil
}

func (r *Repository) listSectionsForSlug(ctx context.Context, slug string) ([]models.PageSection, error) {
	rows, err := r.pool.Query(ctx, `
		SELECT ps.id, ps.section_key, ps.heading, ps.content, ps.sort_order
		FROM page_sections ps
		JOIN pages p ON p.id = ps.page_id
		WHERE p.slug = $1
		ORDER BY ps.sort_order, ps.section_key
	`, slug)
	if err != nil {
		return nil, fmt.Errorf("list sections: %w", err)
	}
	defer rows.Close()

	var sections []models.PageSection
	for rows.Next() {
		var s models.PageSection
		if err := rows.Scan(&s.ID, &s.SectionKey, &s.Heading, &s.Content, &s.SortOrder); err != nil {
			return nil, fmt.Errorf("scan section: %w", err)
		}
		sections = append(sections, s)
	}

	return sections, rows.Err()
}

func (r *Repository) UpdatePage(ctx context.Context, slug string, input models.UpdatePageInput, updatedBy uuid.UUID) (*models.Page, error) {
	title := input.Title
	published := input.Published

	_, err := r.pool.Exec(ctx, `
		UPDATE pages
		SET
			title = COALESCE($2, title),
			published = COALESCE($3, published),
			updated_at = NOW(),
			updated_by = $4
		WHERE slug = $1
	`, slug, title, published, updatedBy)
	if err != nil {
		return nil, fmt.Errorf("update page: %w", err)
	}

	return r.GetPageBySlug(ctx, slug, true)
}

func (r *Repository) UpsertSections(ctx context.Context, slug string, sections []models.UpsertSectionInput, updatedBy uuid.UUID) (*models.Page, error) {
	tx, err := r.pool.Begin(ctx)
	if err != nil {
		return nil, fmt.Errorf("begin tx: %w", err)
	}
	defer tx.Rollback(ctx)

	var pageID uuid.UUID
	err = tx.QueryRow(ctx, `SELECT id FROM pages WHERE slug = $1`, slug).Scan(&pageID)
	if err != nil {
		if errors.Is(err, pgx.ErrNoRows) {
			return nil, ErrNotFound
		}
		return nil, fmt.Errorf("get page id: %w", err)
	}

	for _, section := range sections {
		if section.ID != nil {
			_, err = tx.Exec(ctx, `
				UPDATE page_sections
				SET section_key = $2, heading = $3, content = $4, sort_order = $5
				WHERE id = $1 AND page_id = $6
			`, *section.ID, section.SectionKey, section.Heading, section.Content, section.SortOrder, pageID)
		} else {
			_, err = tx.Exec(ctx, `
				INSERT INTO page_sections (page_id, section_key, heading, content, sort_order)
				VALUES ($1, $2, $3, $4, $5)
				ON CONFLICT (page_id, section_key) DO UPDATE
				SET heading = EXCLUDED.heading, content = EXCLUDED.content, sort_order = EXCLUDED.sort_order
			`, pageID, section.SectionKey, section.Heading, section.Content, section.SortOrder)
		}
		if err != nil {
			return nil, fmt.Errorf("upsert section: %w", err)
		}
	}

	_, err = tx.Exec(ctx, `
		UPDATE pages SET updated_at = NOW(), updated_by = $2 WHERE id = $1
	`, pageID, updatedBy)
	if err != nil {
		return nil, fmt.Errorf("touch page: %w", err)
	}

	if err := tx.Commit(ctx); err != nil {
		return nil, fmt.Errorf("commit tx: %w", err)
	}

	return r.GetPageBySlug(ctx, slug, true)
}

func (r *Repository) CreateSection(ctx context.Context, slug string, input models.CreateSectionInput, updatedBy uuid.UUID) (*models.PageSection, error) {
	var pageID uuid.UUID
	err := r.pool.QueryRow(ctx, `SELECT id FROM pages WHERE slug = $1`, slug).Scan(&pageID)
	if err != nil {
		if errors.Is(err, pgx.ErrNoRows) {
			return nil, ErrNotFound
		}
		return nil, fmt.Errorf("get page id: %w", err)
	}

	var section models.PageSection
	err = r.pool.QueryRow(ctx, `
		INSERT INTO page_sections (page_id, section_key, heading, content, sort_order)
		VALUES ($1, $2, $3, $4, $5)
		RETURNING id, section_key, heading, content, sort_order
	`, pageID, input.SectionKey, input.Heading, input.Content, input.SortOrder).Scan(
		&section.ID, &section.SectionKey, &section.Heading, &section.Content, &section.SortOrder,
	)
	if err != nil {
		return nil, fmt.Errorf("create section: %w", err)
	}

	_, err = r.pool.Exec(ctx, `
		UPDATE pages SET updated_at = NOW(), updated_by = $2 WHERE id = $1
	`, pageID, updatedBy)
	if err != nil {
		return nil, fmt.Errorf("touch page: %w", err)
	}

	return &section, nil
}

func (r *Repository) DeleteSection(ctx context.Context, sectionID uuid.UUID) error {
	tag, err := r.pool.Exec(ctx, `DELETE FROM page_sections WHERE id = $1`, sectionID)
	if err != nil {
		return fmt.Errorf("delete section: %w", err)
	}
	if tag.RowsAffected() == 0 {
		return ErrNotFound
	}
	return nil
}

func (r *Repository) UpsertUser(ctx context.Context, discordID, username, avatarURL string) (*models.User, error) {
	var user models.User
	err := r.pool.QueryRow(ctx, `
		INSERT INTO users (discord_id, username, avatar_url)
		VALUES ($1, $2, $3)
		ON CONFLICT (discord_id) DO UPDATE
		SET username = EXCLUDED.username, avatar_url = EXCLUDED.avatar_url
		RETURNING id, discord_id, username, avatar_url, created_at
	`, discordID, username, avatarURL).Scan(
		&user.ID, &user.DiscordID, &user.Username, &user.AvatarURL, &user.CreatedAt,
	)
	if err != nil {
		return nil, fmt.Errorf("upsert user: %w", err)
	}
	return &user, nil
}

func (r *Repository) GetUserByID(ctx context.Context, id uuid.UUID) (*models.User, error) {
	var user models.User
	err := r.pool.QueryRow(ctx, `
		SELECT id, discord_id, username, avatar_url, created_at
		FROM users WHERE id = $1
	`, id).Scan(&user.ID, &user.DiscordID, &user.Username, &user.AvatarURL, &user.CreatedAt)
	if err != nil {
		if errors.Is(err, pgx.ErrNoRows) {
			return nil, ErrNotFound
		}
		return nil, fmt.Errorf("get user: %w", err)
	}
	return &user, nil
}
