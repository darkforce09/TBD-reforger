package main

import (
	"fmt"
	"log"
	"os"

	"github.com/joho/godotenv"
	"gorm.io/driver/postgres"
	"gorm.io/gorm"
)

func main() {
	if err := godotenv.Load(); err != nil {
		log.Println("No .env file found")
	}

	dsn := "host=localhost user=tbd password=tbd dbname=tbd_reforger port=5434 sslmode=disable TimeZone=UTC"
	db, err := gorm.Open(postgres.Open(dsn), &gorm.Config{})
	if err != nil {
		log.Fatalf("failed to connect database: %v", err)
	}

	sqlBytes, err := os.ReadFile("internal/db/seeds/mock_data.sql")
	if err != nil {
		log.Fatalf("failed to read mock_data.sql: %v", err)
	}

	result := db.Exec(string(sqlBytes))
	if result.Error != nil {
		log.Fatalf("failed to execute seeds: %v", result.Error)
	}

	fmt.Println("Mock data seeded successfully!")
}
