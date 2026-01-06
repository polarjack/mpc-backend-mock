-- Insert a new user into the database
INSERT INTO
    users (email, keycloak_user_id, is_active)
VALUES
    ($1, $2, $3)
RETURNING
    id,
    email,
    keycloak_user_id,
    is_active,
    created_at,
    updated_at,
    deleted_at;
