-- Get user by email (excluding soft-deleted users)
SELECT
    id,
    email,
    keycloak_user_id,
    is_active,
    created_at,
    updated_at,
    deleted_at
FROM
    users
WHERE
    email = $1
    AND deleted_at IS NULL;
