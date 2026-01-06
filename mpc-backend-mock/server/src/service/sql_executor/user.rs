use async_trait::async_trait;
use snafu::ResultExt;
use sqlx::{Executor, Postgres};
use uuid::Uuid;

use crate::{
    entity::User,
    service::error::{self, Result},
};

#[async_trait]
pub trait UserSqlExecutor {
    async fn get_user_by_email(&mut self, email: &str) -> Result<Option<User>>;

    async fn insert_user(
        &mut self,
        email: &str,
        keycloak_user_id: &Uuid,
        is_active: bool,
    ) -> Result<User>;

    async fn get_user_by_id(&mut self, user_id: &Uuid) -> Result<Option<User>>;

    async fn get_user_by_keycloak_id(&mut self, keycloak_user_id: &Uuid) -> Result<Option<User>>;
}

#[async_trait]
impl<E> UserSqlExecutor for E
where
    for<'c> &'c mut E: Executor<'c, Database = Postgres>,
{
    async fn get_user_by_email(&mut self, email: &str) -> Result<Option<User>> {
        let user = sqlx::query_file_as!(User, "sql/user/get_user_by_email.sql", email)
            .fetch_optional(&mut *self)
            .await
            .context(error::GetUserByEmailSnafu)?;

        Ok(user)
    }

    async fn insert_user(
        &mut self,
        email: &str,
        keycloak_user_id: &Uuid,
        is_active: bool,
    ) -> Result<User> {
        let user = sqlx::query_file_as!(
            User,
            "sql/user/insert_user.sql",
            email,
            keycloak_user_id,
            is_active
        )
        .fetch_one(&mut *self)
        .await
        .context(error::InsertUserSnafu)?;

        Ok(user)
    }

    async fn get_user_by_id(&mut self, user_id: &Uuid) -> Result<Option<User>> {
        let user = sqlx::query_file_as!(User, "sql/user/get_user_by_id.sql", user_id)
            .fetch_optional(&mut *self)
            .await
            .context(error::GetUserByIdSnafu)?;

        Ok(user)
    }

    async fn get_user_by_keycloak_id(&mut self, keycloak_user_id: &Uuid) -> Result<Option<User>> {
        let user =
            sqlx::query_file_as!(User, "sql/user/get_user_by_keycloak_id.sql", keycloak_user_id)
                .fetch_optional(&mut *self)
                .await
                .context(error::GetUserByKeycloakIdSnafu)?;

        Ok(user)
    }
}
