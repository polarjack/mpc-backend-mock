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
    keycloak_user_id = $1
    AND deleted_at IS NULL;
