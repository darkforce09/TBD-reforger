// Command import-registry-items upserts a modpack's Virtual Arsenal catalog
// (T-068) from a registry-items JSON envelope produced by the TBD-Content
// Workbench export (packages/tbd-schema/registry/registry-items.workbench.json).
//
// Usage:
//
//	go run ./cmd/import-registry-items --file <path-to-registry-items.json>
//
// Rows are upserted on (modpack_id, resource_name): re-running with an updated
// export refreshes display_name/category/icon_url/kind/sort_order in place.
package main

import (
	"encoding/json"
	"flag"
	"fmt"
	"log"
	"os"

	"github.com/google/uuid"
	"github.com/joho/godotenv"
	"gorm.io/driver/postgres"
	"gorm.io/gorm"
	"gorm.io/gorm/clause"

	"github.com/tbd-milsim/reforger-backend/internal/models"
)

// registryItemsEnvelope mirrors packages/tbd-schema/schema/registry-items.schema.json.
type registryItemsEnvelope struct {
	RegistryItemsVersion string `json:"registryItemsVersion"`
	ModpackID            string `json:"modpackId"`
	GeneratedAt          string `json:"generatedAt"`
	Items                []struct {
		ResourceName string `json:"resource_name"`
		DisplayName  string `json:"display_name"`
		Category     string `json:"category"`
		IconURL      string `json:"icon_url"`
		Kind         string `json:"kind"`
	} `json:"items"`
}

func main() {
	file := flag.String("file", "", "path to a registry-items JSON envelope (required)")
	flag.Parse()
	if *file == "" {
		log.Fatal("--file is required")
	}

	if err := godotenv.Load(); err != nil {
		log.Println("No .env file found")
	}

	raw, err := os.ReadFile(*file)
	if err != nil {
		log.Fatalf("read %s: %v", *file, err)
	}
	var env registryItemsEnvelope
	if err := json.Unmarshal(raw, &env); err != nil {
		log.Fatalf("parse %s: %v", *file, err)
	}
	modpackID, err := uuid.Parse(env.ModpackID)
	if err != nil {
		log.Fatalf("invalid modpackId %q: %v", env.ModpackID, err)
	}
	if len(env.Items) == 0 {
		log.Fatal("envelope has no items")
	}

	dsn := os.Getenv("DATABASE_URL")
	if dsn == "" {
		dsn = "host=localhost user=tbd password=tbd dbname=tbd_reforger port=5434 sslmode=disable TimeZone=UTC"
	}
	db, err := gorm.Open(postgres.Open(dsn), &gorm.Config{})
	if err != nil {
		log.Fatalf("connect database: %v", err)
	}

	rows := make([]models.RegistryItem, 0, len(env.Items))
	for i, it := range env.Items {
		rows = append(rows, models.RegistryItem{
			ModpackID:    modpackID,
			ResourceName: it.ResourceName,
			DisplayName:  it.DisplayName,
			Category:     it.Category,
			IconURL:      it.IconURL,
			Kind:         it.Kind,
			SortOrder:    i + 1,
		})
	}

	// Upsert on the (modpack_id, resource_name) unique index.
	res := db.Clauses(clause.OnConflict{
		Columns: []clause.Column{{Name: "modpack_id"}, {Name: "resource_name"}},
		DoUpdates: clause.AssignmentColumns([]string{
			"display_name", "category", "icon_url", "kind", "sort_order", "updated_at",
		}),
	}).Create(&rows)
	if res.Error != nil {
		log.Fatalf("upsert registry items: %v", res.Error)
	}

	fmt.Printf("Imported %d registry items for modpack %s\n", len(rows), modpackID)
}
