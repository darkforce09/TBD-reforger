package handlers

import (
	"fmt"
	"net/http"
	"regexp"
	"testing"
	"time"

	"github.com/google/uuid"

	"github.com/tbd-milsim/reforger-backend/internal/models"
)

// resourceNameRe is the Enfusion ResourceName GUID prefix contract (A4).
var resourceNameRe = regexp.MustCompile(`^\{[0-9A-F]{16}\}`)

func TestRegistryIntegration(t *testing.T) {
	r, h, gdb := setupIT(t)

	mmID := fmt.Sprintf("itest-mm-%d", time.Now().UnixNano())
	enlistedID := fmt.Sprintf("itest-enl-%d", time.Now().UnixNano())
	modpackID := uuid.New()

	t.Cleanup(func() {
		gdb.Where("modpack_id = ?", modpackID).Delete(&models.RegistryItem{})
		gdb.Where("id = ?", modpackID).Delete(&models.Modpack{})
		gdb.Unscoped().Where("discord_id IN ?", []string{mmID, enlistedID}).Delete(&models.User{})
	})

	gdb.Create(&models.User{DiscordID: mmID, Username: "MM Mike", Role: models.RoleMissionMaker})
	gdb.Create(&models.User{DiscordID: enlistedID, Username: "Enlisted Joe", Role: models.RoleEnlisted})
	mmTok, _, _ := h.JWT().IssueAccess(mmID, "mission_maker", false)
	enlTok, _, _ := h.JWT().IssueAccess(enlistedID, "enlisted", false)

	if err := gdb.Create(&models.Modpack{
		ID: modpackID, Name: "ITest Pack", Version: "9.9", TotalSizeBytes: 1, IsCurrent: false,
	}).Error; err != nil {
		t.Fatalf("seed modpack: %v", err)
	}
	items := []models.RegistryItem{
		{ModpackID: modpackID, ResourceName: "{26A9756790131354}Prefabs/Characters/Char_A.et", DisplayName: "Char A", Category: "NATO/Rifleman", Kind: "character", SortOrder: 1},
		{ModpackID: modpackID, ResourceName: "{3E413771E1834D2F}Prefabs/Weapons/Rifle_X.et", DisplayName: "Rifle X", Category: "NATO/Weapons/Primary", Kind: "gear_primary", SortOrder: 2},
		{ModpackID: modpackID, ResourceName: "{4B57C11AA5161760}Prefabs/Vests/Vest_X.et", DisplayName: "Vest X", Category: "NATO/Vest", Kind: "gear_vest", SortOrder: 3},
	}
	if err := gdb.Create(&items).Error; err != nil {
		t.Fatalf("seed registry items: %v", err)
	}

	base := "/api/v1/registry?modpack=" + modpackID.String()

	// --- 401: no bearer ---
	if w := do(r, "GET", base, reqOpt{}); w.Code != http.StatusUnauthorized {
		t.Fatalf("unauthenticated GET /registry = %d, want 401", w.Code)
	}

	// --- 403: enlisted is below mission_maker ---
	if w := do(r, "GET", base, reqOpt{bearer: enlTok}); w.Code != http.StatusForbidden {
		t.Fatalf("enlisted GET /registry = %d, want 403", w.Code)
	}

	// --- 200: mission_maker gets the catalog + etag ---
	w := do(r, "GET", base, reqOpt{bearer: mmTok})
	if w.Code != http.StatusOK {
		t.Fatalf("GET /registry = %d, body=%s", w.Code, w.Body.String())
	}
	var resp struct {
		Data           []models.RegistryItem `json:"data"`
		ETag           string                `json:"etag"`
		ModpackID      string                `json:"modpack_id"`
		ModpackVersion string                `json:"modpack_version"`
	}
	mustJSON(t, w, &resp)
	if len(resp.Data) != 3 {
		t.Fatalf("data length = %d, want 3", len(resp.Data))
	}
	if resp.ETag == "" {
		t.Fatal("missing etag")
	}
	if resp.ModpackID != modpackID.String() {
		t.Fatalf("modpack_id = %q, want %q", resp.ModpackID, modpackID)
	}
	if resp.ModpackVersion != "9.9" {
		t.Fatalf("modpack_version = %q, want 9.9", resp.ModpackVersion)
	}
	first := resp.Data[0]
	if first.ResourceName == "" || first.DisplayName == "" || first.Category == "" || first.Kind == "" {
		t.Fatalf("row missing required fields: %+v", first)
	}
	if !resourceNameRe.MatchString(first.ResourceName) {
		t.Fatalf("resource_name %q does not match GUID prefix", first.ResourceName)
	}
	// ETag header should match the body etag.
	if hdr := w.Header().Get("ETag"); hdr != resp.ETag {
		t.Fatalf("ETag header %q != body etag %q", hdr, resp.ETag)
	}

	// --- 304: If-None-Match with the same etag ---
	w304 := do(r, "GET", base, reqOpt{bearer: mmTok, ifNoneMatch: resp.ETag})
	if w304.Code != http.StatusNotModified {
		t.Fatalf("If-None-Match GET /registry = %d, want 304", w304.Code)
	}

	// --- 404: unknown modpack ---
	if w := do(r, "GET", "/api/v1/registry?modpack=00000000-0000-0000-0000-000000000000", reqOpt{bearer: mmTok}); w.Code != http.StatusNotFound {
		t.Fatalf("unknown modpack GET /registry = %d, want 404", w.Code)
	}
}
