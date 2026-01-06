-- Enable UUID extension
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- Create users table for MPC backend with Keycloak integration
-- This table stores user information and links to Keycloak user IDs
CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    email VARCHAR(255) NOT NULL UNIQUE,
    keycloak_user_id UUID NOT NULL UNIQUE,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    deleted_at TIMESTAMP WITH TIME ZONE
);

-- Create indexes for efficient querying
CREATE INDEX idx_users_email ON users(email)
WHERE
    deleted_at IS NULL;

CREATE INDEX idx_users_keycloak_user_id ON users(keycloak_user_id)
WHERE
    deleted_at IS NULL;

CREATE INDEX idx_users_active ON users(is_active)
WHERE
    deleted_at IS NULL;

CREATE INDEX idx_users_created_at ON users(created_at);

-- Add comment to table
COMMENT ON TABLE users IS 'User accounts with Keycloak integration for authentication';

COMMENT ON COLUMN users.id IS 'Primary key, auto-generated UUID';

COMMENT ON COLUMN users.email IS 'User email address, must be unique';

COMMENT ON COLUMN users.keycloak_user_id IS 'Reference to Keycloak user ID';

COMMENT ON COLUMN users.is_active IS 'Whether the user account is active';

COMMENT ON COLUMN users.created_at IS 'Timestamp when user was created';

COMMENT ON COLUMN users.updated_at IS 'Timestamp when user was last updated';

COMMENT ON COLUMN users.deleted_at IS 'Timestamp when user was soft deleted, NULL if not deleted';

-- Create function to automatically update updated_at timestamp
CREATE
OR REPLACE FUNCTION update_updated_at_column() RETURNS TRIGGER AS
$$
BEGIN
NEW.updated_at = NOW();

RETURN NEW;

END;

$$
LANGUAGE plpgsql;

-- Create trigger to automatically update updated_at on row updates
CREATE TRIGGER update_users_updated_at BEFORE
UPDATE
    ON users FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
