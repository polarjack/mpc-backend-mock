-- Revert users table creation
-- Drop trigger
DROP TRIGGER IF EXISTS update_users_updated_at ON users;

-- Drop function
DROP FUNCTION IF EXISTS update_updated_at_column();

-- Drop indexes (will be dropped automatically with table, but being explicit)
DROP INDEX IF EXISTS idx_users_created_at;

DROP INDEX IF EXISTS idx_users_active;

DROP INDEX IF EXISTS idx_users_keycloak_user_id;

DROP INDEX IF EXISTS idx_users_email;

-- Drop table
DROP TABLE IF EXISTS users;
