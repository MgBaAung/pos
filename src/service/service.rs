use crate::database::Database;
use crate::error::{AppError, AppResult};
use crate::models::{
    AssignUserToBranchRequest, Branch, BranchComparison, BranchInventorySummary, BranchPerformance,
    CreateBranchRequest, UpdateBranchRequest, UserBranchAccess, UserBranchAssignment,
};
use std::sync::Arc;

pub struct BranchService {
    db: Arc<Database>,
}

impl BranchService {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    /// Create a new branch
    pub async fn create_branch(&self, request: CreateBranchRequest) -> AppResult<Branch> {
        // Check if branch code already exists
        let existing = sqlx::query_scalar::<_, i32>("SELECT id FROM branches WHERE code = $1")
            .bind(&request.code)
            .fetch_optional(&self.db.pool)
            .await?;

        if existing.is_some() {
            return Err(AppError::Conflict(format!(
                "Branch code {} already exists",
                request.code
            )));
        }

        // Parse time strings if provided
        let opening_time = request
            .opening_time
            .and_then(|t| chrono::NaiveTime::parse_from_str(&t, "%H:%M").ok());
        let closing_time = request
            .closing_time
            .and_then(|t| chrono::NaiveTime::parse_from_str(&t, "%H:%M").ok());

        let settings = request.settings.unwrap_or_else(|| serde_json::json!({}));
        let is_main_branch = request.is_main_branch.unwrap_or(false);

        let branch = sqlx::query_as::<_, Branch>(
            r#"
            INSERT INTO branches (name, code, address, phone, email, manager_name, manager_phone, 
                               tax_id, business_license, opening_time, closing_time, is_main_branch, settings)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
            RETURNING *
            "#,
        )
        .bind(&request.name)
        .bind(&request.code)
        .bind(&request.address)
        .bind(&request.phone)
        .bind(&request.email)
        .bind(&request.manager_name)
        .bind(&request.manager_phone)
        .bind(&request.tax_id)
        .bind(&request.business_license)
        .bind(opening_time)
        .bind(closing_time)
        .bind(is_main_branch)
        .bind(settings)
        .fetch_one(&self.db.pool)
        .await?;

        tracing::info!("Branch created: {} ({})", branch.name, branch.code);
        Ok(branch)
    }

    /// Get all branches
    pub async fn get_all_branches(&self, include_inactive: bool) -> AppResult<Vec<Branch>> {
        let branches = if include_inactive {
            sqlx::query_as::<_, Branch>("SELECT * FROM branches ORDER BY name")
                .fetch_all(&self.db.pool)
                .await?
        } else {
            sqlx::query_as::<_, Branch>(
                "SELECT * FROM branches WHERE is_active = TRUE ORDER BY name",
            )
            .fetch_all(&self.db.pool)
            .await?
        };

        Ok(branches)
    }

    /// Get branch by ID
    pub async fn get_branch_by_id(&self, branch_id: i32) -> AppResult<Branch> {
        let branch = sqlx::query_as::<_, Branch>("SELECT * FROM branches WHERE id = $1")
            .bind(branch_id)
            .fetch_optional(&self.db.pool)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Branch {} not found", branch_id)))?;

        Ok(branch)
    }

    /// Get branch by code
    pub async fn get_branch_by_code(&self, code: &str) -> AppResult<Branch> {
        let branch = sqlx::query_as::<_, Branch>("SELECT * FROM branches WHERE code = $1")
            .bind(code)
            .fetch_optional(&self.db.pool)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Branch with code {} not found", code)))?;

        Ok(branch)
    }

    /// Update branch
    pub async fn update_branch(
        &self,
        branch_id: i32,
        request: UpdateBranchRequest,
    ) -> AppResult<Branch> {
        // Check if branch exists
        let existing = self.get_branch_by_id(branch_id).await?;

        // Parse time strings if provided
        let opening_time = request
            .opening_time
            .and_then(|t| chrono::NaiveTime::parse_from_str(&t, "%H:%M").ok());
        let closing_time = request
            .closing_time
            .and_then(|t| chrono::NaiveTime::parse_from_str(&t, "%H:%M").ok());

        // Build dynamic update query
        let mut query = String::from("UPDATE branches SET ");
        let mut updates = Vec::new();
        let mut param_count = 1;

        if let Some(_name) = &request.name {
            updates.push(format!("name = ${}", param_count));
            param_count += 1;
        }
        if let Some(_address) = &request.address {
            updates.push(format!("address = ${}", param_count));
            param_count += 1;
        }
        if let Some(_phone) = &request.phone {
            updates.push(format!("phone = ${}", param_count));
            param_count += 1;
        }
        if let Some(_email) = &request.email {
            updates.push(format!("email = ${}", param_count));
            param_count += 1;
        }
        if let Some(_manager_name) = &request.manager_name {
            updates.push(format!("manager_name = ${}", param_count));
            param_count += 1;
        }
        if let Some(_manager_phone) = &request.manager_phone {
            updates.push(format!("manager_phone = ${}", param_count));
            param_count += 1;
        }
        if let Some(_tax_id) = &request.tax_id {
            updates.push(format!("tax_id = ${}", param_count));
            param_count += 1;
        }
        if let Some(_business_license) = &request.business_license {
            updates.push(format!("business_license = ${}", param_count));
            param_count += 1;
        }
        if opening_time.is_some() {
            updates.push(format!("opening_time = ${}", param_count));
            param_count += 1;
        }
        if closing_time.is_some() {
            updates.push(format!("closing_time = ${}", param_count));
            param_count += 1;
        }
        if let Some(_is_active) = request.is_active {
            updates.push(format!("is_active = ${}", param_count));
            param_count += 1;
        }
        if let Some(_is_main_branch) = request.is_main_branch {
            updates.push(format!("is_main_branch = ${}", param_count));
            param_count += 1;
        }
        if let Some(_settings) = &request.settings {
            updates.push(format!("settings = ${}", param_count));
            param_count += 1;
        }

        if updates.is_empty() {
            return Ok(existing);
        }

        query.push_str(&updates.join(", "));
        query.push_str(&format!(" WHERE id = ${} RETURNING *", param_count));

        let mut query_builder = sqlx::query_as::<_, Branch>(&query);

        if let Some(name) = &request.name {
            query_builder = query_builder.bind(name);
        }
        if let Some(address) = &request.address {
            query_builder = query_builder.bind(address);
        }
        if let Some(phone) = &request.phone {
            query_builder = query_builder.bind(phone);
        }
        if let Some(email) = &request.email {
            query_builder = query_builder.bind(email);
        }
        if let Some(manager_name) = &request.manager_name {
            query_builder = query_builder.bind(manager_name);
        }
        if let Some(manager_phone) = &request.manager_phone {
            query_builder = query_builder.bind(manager_phone);
        }
        if let Some(tax_id) = &request.tax_id {
            query_builder = query_builder.bind(tax_id);
        }
        if let Some(business_license) = &request.business_license {
            query_builder = query_builder.bind(business_license);
        }
        if let Some(ot) = opening_time {
            query_builder = query_builder.bind(ot);
        }
        if let Some(ct) = closing_time {
            query_builder = query_builder.bind(ct);
        }
        if let Some(is_active) = request.is_active {
            query_builder = query_builder.bind(is_active);
        }
        if let Some(is_main_branch) = request.is_main_branch {
            query_builder = query_builder.bind(is_main_branch);
        }
        if let Some(settings) = &request.settings {
            query_builder = query_builder.bind(settings);
        }

        query_builder = query_builder.bind(branch_id);

        let branch = query_builder.fetch_one(&self.db.pool).await?;

        tracing::info!("Branch updated: {} ({})", branch.name, branch.code);
        Ok(branch)
    }

    /// Delete branch (soft delete by setting is_active = false)
    pub async fn delete_branch(&self, branch_id: i32) -> AppResult<()> {
        // Check if branch exists
        let branch = self.get_branch_by_id(branch_id).await?;

        // Prevent deletion of main branch
        if branch.is_main_branch {
            return Err(AppError::Validation(
                "Cannot delete main branch".to_string(),
            ));
        }

        // Check if branch has associated data
        let sales_count =
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM sales WHERE branch_id = $1")
                .bind(branch_id)
                .fetch_one(&self.db.pool)
                .await?;

        if sales_count > 0 {
            return Err(AppError::Validation(format!(
                "Cannot delete branch with {} sales. Archive it instead.",
                sales_count
            )));
        }

        // Soft delete
        sqlx::query("UPDATE branches SET is_active = FALSE WHERE id = $1")
            .bind(branch_id)
            .execute(&self.db.pool)
            .await?;

        tracing::info!("Branch deleted (soft delete): {}", branch.name);
        Ok(())
    }

    /// Get branch performance metrics
    pub async fn get_branch_performance(
        &self,
        branch_id: Option<i32>,
    ) -> AppResult<Vec<BranchPerformance>> {
        let performances = if let Some(id) = branch_id {
            sqlx::query_as::<_, BranchPerformance>(
                "SELECT * FROM branch_performance WHERE branch_id = $1",
            )
            .bind(id)
            .fetch_all(&self.db.pool)
            .await?
        } else {
            sqlx::query_as::<_, BranchPerformance>("SELECT * FROM branch_performance")
                .fetch_all(&self.db.pool)
                .await?
        };

        Ok(performances)
    }

    /// Get branch comparison metrics
    pub async fn get_branch_comparison(&self) -> AppResult<Vec<BranchComparison>> {
        let comparisons = sqlx::query_as::<_, BranchComparison>("SELECT * FROM branch_comparison")
            .fetch_all(&self.db.pool)
            .await?;

        Ok(comparisons)
    }

    /// Get branch inventory summary
    pub async fn get_branch_inventory_summary(
        &self,
        branch_id: Option<i32>,
    ) -> AppResult<Vec<BranchInventorySummary>> {
        let summaries = if let Some(id) = branch_id {
            sqlx::query_as::<_, BranchInventorySummary>(
                "SELECT * FROM branch_inventory_summary WHERE branch_id = $1",
            )
            .bind(id)
            .fetch_all(&self.db.pool)
            .await?
        } else {
            sqlx::query_as::<_, BranchInventorySummary>("SELECT * FROM branch_inventory_summary")
                .fetch_all(&self.db.pool)
                .await?
        };

        Ok(summaries)
    }

    /// Assign user to branch
    pub async fn assign_user_to_branch(
        &self,
        request: AssignUserToBranchRequest,
        assigned_by: i32,
    ) -> AppResult<UserBranchAssignment> {
        // Check if user exists
        let user_exists = sqlx::query_scalar::<_, i32>("SELECT id FROM users WHERE id = $1")
            .bind(request.user_id)
            .fetch_optional(&self.db.pool)
            .await?;

        if user_exists.is_none() {
            return Err(AppError::NotFound(format!(
                "User {} not found",
                request.user_id
            )));
        }

        // Check if branch exists
        let branch_exists = sqlx::query_scalar::<_, i32>("SELECT id FROM branches WHERE id = $1")
            .bind(request.branch_id)
            .fetch_optional(&self.db.pool)
            .await?;

        if branch_exists.is_none() {
            return Err(AppError::NotFound(format!(
                "Branch {} not found",
                request.branch_id
            )));
        }

        // If setting as primary, remove primary from other branches
        if request.is_primary.unwrap_or(false) {
            sqlx::query("UPDATE user_branch_assignments SET is_primary = FALSE WHERE user_id = $1")
                .bind(request.user_id)
                .execute(&self.db.pool)
                .await?;
        }

        let assignment = sqlx::query_as::<_, UserBranchAssignment>(
            r#"
            INSERT INTO user_branch_assignments (user_id, branch_id, assigned_by, is_primary)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (user_id, branch_id) 
            DO UPDATE SET assigned_by = $3, is_primary = $4
            RETURNING *
            "#,
        )
        .bind(request.user_id)
        .bind(request.branch_id)
        .bind(assigned_by)
        .bind(request.is_primary.unwrap_or(false))
        .fetch_one(&self.db.pool)
        .await?;

        tracing::info!(
            "User {} assigned to branch {}",
            request.user_id,
            request.branch_id
        );
        Ok(assignment)
    }

    /// Remove user from branch
    pub async fn remove_user_from_branch(&self, user_id: i32, branch_id: i32) -> AppResult<()> {
        let result = sqlx::query(
            "DELETE FROM user_branch_assignments WHERE user_id = $1 AND branch_id = $2",
        )
        .bind(user_id)
        .bind(branch_id)
        .execute(&self.db.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!(
                "User {} not assigned to branch {}",
                user_id, branch_id
            )));
        }

        tracing::info!("User {} removed from branch {}", user_id, branch_id);
        Ok(())
    }

    /// Get user branch access
    pub async fn get_user_branch_access(&self, user_id: i32) -> AppResult<UserBranchAccess> {
        let access = sqlx::query_as::<_, UserBranchAccess>(
            "SELECT * FROM user_branch_access WHERE user_id = $1",
        )
        .bind(user_id)
        .fetch_optional(&self.db.pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("User {} not found", user_id)))?;

        Ok(access)
    }

    /// Get all user branch assignments
    pub async fn get_user_branch_assignments(
        &self,
        user_id: i32,
    ) -> AppResult<Vec<UserBranchAssignment>> {
        let assignments = sqlx::query_as::<_, UserBranchAssignment>(
            "SELECT * FROM user_branch_assignments WHERE user_id = $1 ORDER BY assigned_at DESC",
        )
        .bind(user_id)
        .fetch_all(&self.db.pool)
        .await?;

        Ok(assignments)
    }

    /// Get branch users
    pub async fn get_branch_users(&self, branch_id: i32) -> AppResult<Vec<crate::models::User>> {
        let users = sqlx::query_as::<_, crate::models::User>(
            r#"
            SELECT u.* FROM users u
            WHERE u.branch_id = $1 OR u.id IN (
                SELECT user_id FROM user_branch_assignments WHERE branch_id = $1
            )
            ORDER BY u.full_name
            "#,
        )
        .bind(branch_id)
        .fetch_all(&self.db.pool)
        .await?;

        Ok(users)
    }
}
