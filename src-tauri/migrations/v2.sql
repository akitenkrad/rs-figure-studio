-- V2: Add 'generated' status support for AI generation pipeline
-- sprites.status is TEXT and already accepts any value.
-- This migration adds concept_image_path to characters for AI generation reference.

ALTER TABLE characters ADD COLUMN concept_image_path TEXT;
