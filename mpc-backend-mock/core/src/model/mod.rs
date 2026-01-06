// include the model for api input, output. EX: CreateUserRequest,
// CreateUserResponse....

use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

#[derive(Clone, Debug, Serialize, Deserialize, IntoParams, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Pagination {
    pub page: Option<u32>,
    pub limit: Option<u32>,
}
