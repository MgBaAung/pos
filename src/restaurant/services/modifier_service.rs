use crate::database::Database;
use crate::error::AppError;
use crate::restaurant::domain::modifiers::{
    CreateModifierGroupRequest, CreateModifierRequest, LinkProductToModifierGroupRequest, Modifier,
    ModifierGroup, ModifierGroupResponse, ModifierResponse, UpdateModifierGroupRequest,
    UpdateModifierRequest,
};
use sqlx::PgPool;

pub struct ModifierService {
    pool: PgPool,
}

impl ModifierService {
    pub fn new(database: &Database) -> Self {
        Self {
            pool: database.pool.clone(),
        }
    }

    // Modifier Group Operations
    pub async fn create_modifier_group(
        &self,
        request: CreateModifierGroupRequest,
    ) -> Result<ModifierGroupResponse, AppError> {
        let group = sqlx::query_as::<_, ModifierGroup>(
            r#"
            INSERT INTO modifier_groups (name, description, is_required, min_selections, max_selections, display_order)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING *
            "#
        )
        .bind(&request.name)
        .bind(&request.description)
        .bind(request.is_required.unwrap_or(false))
        .bind(request.min_selections.unwrap_or(0))
        .bind(request.max_selections.unwrap_or(1))
        .bind(request.display_order.unwrap_or(0))
        .fetch_one(&self.pool)
        .await?;

        self.get_modifier_group_with_modifiers(group.id).await
    }

    pub async fn get_modifier_group(&self, id: i32) -> Result<ModifierGroupResponse, AppError> {
        self.get_modifier_group_with_modifiers(id).await
    }

    pub async fn list_modifier_groups(
        &self,
        is_active: Option<bool>,
    ) -> Result<Vec<ModifierGroupResponse>, AppError> {
        let groups = if let Some(is_active) = is_active {
            sqlx::query_as::<_, ModifierGroup>(
                "SELECT * FROM modifier_groups WHERE is_active = $1 ORDER BY display_order, name",
            )
            .bind(is_active)
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query_as::<_, ModifierGroup>(
                "SELECT * FROM modifier_groups ORDER BY display_order, name",
            )
            .fetch_all(&self.pool)
            .await?
        };

        let mut responses = Vec::new();
        for group in groups {
            let modifiers = self.get_modifiers_for_group(group.id).await?;
            responses.push(ModifierGroupResponse {
                id: group.id,
                name: group.name,
                description: group.description,
                is_required: group.is_required,
                min_selections: group.min_selections,
                max_selections: group.max_selections,
                display_order: group.display_order,
                is_active: group.is_active,
                modifiers,
                created_at: group.created_at,
                updated_at: group.updated_at,
            });
        }

        Ok(responses)
    }

    pub async fn update_modifier_group(
        &self,
        id: i32,
        request: UpdateModifierGroupRequest,
    ) -> Result<ModifierGroupResponse, AppError> {
        let mut updates = Vec::new();
        let mut param_count = 0;

        if request.name.is_some() {
            param_count += 1;
            updates.push(format!("name = ${}", param_count));
        }
        if request.description.is_some() {
            param_count += 1;
            updates.push(format!("description = ${}", param_count));
        }
        if request.is_required.is_some() {
            param_count += 1;
            updates.push(format!("is_required = ${}", param_count));
        }
        if request.min_selections.is_some() {
            param_count += 1;
            updates.push(format!("min_selections = ${}", param_count));
        }
        if request.max_selections.is_some() {
            param_count += 1;
            updates.push(format!("max_selections = ${}", param_count));
        }
        if request.display_order.is_some() {
            param_count += 1;
            updates.push(format!("display_order = ${}", param_count));
        }
        if request.is_active.is_some() {
            param_count += 1;
            updates.push(format!("is_active = ${}", param_count));
        }

        if updates.is_empty() {
            return self.get_modifier_group(id).await;
        }

        let query = format!(
            "UPDATE modifier_groups SET {} WHERE id = ${} RETURNING *",
            updates.join(", "),
            param_count + 1
        );

        let mut query_builder = sqlx::query_as::<_, ModifierGroup>(&query);

        if let Some(name) = request.name {
            query_builder = query_builder.bind(name);
        }
        if let Some(description) = request.description {
            query_builder = query_builder.bind(description);
        }
        if let Some(is_required) = request.is_required {
            query_builder = query_builder.bind(is_required);
        }
        if let Some(min_selections) = request.min_selections {
            query_builder = query_builder.bind(min_selections);
        }
        if let Some(max_selections) = request.max_selections {
            query_builder = query_builder.bind(max_selections);
        }
        if let Some(display_order) = request.display_order {
            query_builder = query_builder.bind(display_order);
        }
        if let Some(is_active) = request.is_active {
            query_builder = query_builder.bind(is_active);
        }
        query_builder = query_builder.bind(id);

        let group = query_builder.fetch_one(&self.pool).await.map_err(|e| {
            if e.to_string().contains("no rows returned") {
                AppError::NotFound("Modifier group not found".to_string())
            } else {
                AppError::Database(e)
            }
        })?;

        self.get_modifier_group_with_modifiers(group.id).await
    }

    pub async fn delete_modifier_group(&self, id: i32) -> Result<(), AppError> {
        let result = sqlx::query("DELETE FROM modifier_groups WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound("Modifier group not found".to_string()));
        }

        Ok(())
    }

    // Modifier Operations
    pub async fn create_modifier(
        &self,
        request: CreateModifierRequest,
    ) -> Result<ModifierResponse, AppError> {
        let modifier = sqlx::query_as::<_, Modifier>(
            r#"
            INSERT INTO modifiers (group_id, name, description, price_adjustment, display_order)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING *
            "#,
        )
        .bind(request.group_id)
        .bind(&request.name)
        .bind(&request.description)
        .bind(
            request
                .price_adjustment
                .unwrap_or(rust_decimal::Decimal::ZERO),
        )
        .bind(request.display_order.unwrap_or(0))
        .fetch_one(&self.pool)
        .await?;

        Ok(ModifierResponse::from(modifier))
    }

    pub async fn get_modifier(&self, id: i32) -> Result<ModifierResponse, AppError> {
        let modifier = sqlx::query_as::<_, Modifier>("SELECT * FROM modifiers WHERE id = $1")
            .bind(id)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| {
                if e.to_string().contains("no rows returned") {
                    AppError::NotFound("Modifier not found".to_string())
                } else {
                    AppError::Database(e)
                }
            })?;

        Ok(ModifierResponse::from(modifier))
    }

    pub async fn update_modifier(
        &self,
        id: i32,
        request: UpdateModifierRequest,
    ) -> Result<ModifierResponse, AppError> {
        let mut updates = Vec::new();
        let mut param_count = 0;

        if request.name.is_some() {
            param_count += 1;
            updates.push(format!("name = ${}", param_count));
        }
        if request.description.is_some() {
            param_count += 1;
            updates.push(format!("description = ${}", param_count));
        }
        if request.price_adjustment.is_some() {
            param_count += 1;
            updates.push(format!("price_adjustment = ${}", param_count));
        }
        if request.is_available.is_some() {
            param_count += 1;
            updates.push(format!("is_available = ${}", param_count));
        }
        if request.display_order.is_some() {
            param_count += 1;
            updates.push(format!("display_order = ${}", param_count));
        }

        if updates.is_empty() {
            return self.get_modifier(id).await;
        }

        let query = format!(
            "UPDATE modifiers SET {} WHERE id = ${} RETURNING *",
            updates.join(", "),
            param_count + 1
        );

        let mut query_builder = sqlx::query_as::<_, Modifier>(&query);

        if let Some(name) = request.name {
            query_builder = query_builder.bind(name);
        }
        if let Some(description) = request.description {
            query_builder = query_builder.bind(description);
        }
        if let Some(price_adjustment) = request.price_adjustment {
            query_builder = query_builder.bind(price_adjustment);
        }
        if let Some(is_available) = request.is_available {
            query_builder = query_builder.bind(is_available);
        }
        if let Some(display_order) = request.display_order {
            query_builder = query_builder.bind(display_order);
        }
        query_builder = query_builder.bind(id);

        let modifier = query_builder.fetch_one(&self.pool).await.map_err(|e| {
            if e.to_string().contains("no rows returned") {
                AppError::NotFound("Modifier not found".to_string())
            } else {
                AppError::Database(e)
            }
        })?;

        Ok(ModifierResponse::from(modifier))
    }

    pub async fn delete_modifier(&self, id: i32) -> Result<(), AppError> {
        let result = sqlx::query("DELETE FROM modifiers WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound("Modifier not found".to_string()));
        }

        Ok(())
    }

    // Product-Modifier Group Linking
    pub async fn link_product_to_modifier_group(
        &self,
        request: LinkProductToModifierGroupRequest,
    ) -> Result<(), AppError> {
        sqlx::query(
            r#"
            INSERT INTO product_modifier_groups (product_id, modifier_group_id, is_required)
            VALUES ($1, $2, $3)
            ON CONFLICT (product_id, modifier_group_id) 
            DO UPDATE SET is_required = $3
            "#,
        )
        .bind(request.product_id)
        .bind(request.modifier_group_id)
        .bind(request.is_required.unwrap_or(false))
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn unlink_product_from_modifier_group(
        &self,
        product_id: i32,
        modifier_group_id: i32,
    ) -> Result<(), AppError> {
        let result = sqlx::query(
            "DELETE FROM product_modifier_groups WHERE product_id = $1 AND modifier_group_id = $2",
        )
        .bind(product_id)
        .bind(modifier_group_id)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(
                "Product-modifier group link not found".to_string(),
            ));
        }

        Ok(())
    }

    pub async fn get_product_modifier_groups(
        &self,
        product_id: i32,
    ) -> Result<Vec<ModifierGroupResponse>, AppError> {
        let groups = sqlx::query_as::<_, ModifierGroup>(
            r#"
            SELECT mg.* FROM modifier_groups mg
            INNER JOIN product_modifier_groups pmg ON mg.id = pmg.modifier_group_id
            WHERE pmg.product_id = $1 AND mg.is_active = TRUE
            ORDER BY mg.display_order, mg.name
            "#,
        )
        .bind(product_id)
        .fetch_all(&self.pool)
        .await?;

        let mut responses = Vec::new();
        for group in groups {
            let modifiers = self.get_modifiers_for_group(group.id).await?;
            responses.push(ModifierGroupResponse {
                id: group.id,
                name: group.name,
                description: group.description,
                is_required: group.is_required,
                min_selections: group.min_selections,
                max_selections: group.max_selections,
                display_order: group.display_order,
                is_active: group.is_active,
                modifiers,
                created_at: group.created_at,
                updated_at: group.updated_at,
            });
        }

        Ok(responses)
    }

    // Helper methods
    async fn get_modifier_group_with_modifiers(
        &self,
        id: i32,
    ) -> Result<ModifierGroupResponse, AppError> {
        let group =
            sqlx::query_as::<_, ModifierGroup>("SELECT * FROM modifier_groups WHERE id = $1")
                .bind(id)
                .fetch_one(&self.pool)
                .await
                .map_err(|e| {
                    if e.to_string().contains("no rows returned") {
                        AppError::NotFound("Modifier group not found".to_string())
                    } else {
                        AppError::Database(e)
                    }
                })?;

        let modifiers = self.get_modifiers_for_group(group.id).await?;

        Ok(ModifierGroupResponse {
            id: group.id,
            name: group.name,
            description: group.description,
            is_required: group.is_required,
            min_selections: group.min_selections,
            max_selections: group.max_selections,
            display_order: group.display_order,
            is_active: group.is_active,
            modifiers,
            created_at: group.created_at,
            updated_at: group.updated_at,
        })
    }

    async fn get_modifiers_for_group(
        &self,
        group_id: i32,
    ) -> Result<Vec<ModifierResponse>, AppError> {
        let modifiers = sqlx::query_as::<_, Modifier>(
            "SELECT * FROM modifiers WHERE group_id = $1 ORDER BY display_order, name",
        )
        .bind(group_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(modifiers.into_iter().map(ModifierResponse::from).collect())
    }
}

impl From<Modifier> for ModifierResponse {
    fn from(modifier: Modifier) -> Self {
        ModifierResponse {
            id: modifier.id,
            group_id: modifier.group_id,
            name: modifier.name,
            description: modifier.description,
            price_adjustment: modifier.price_adjustment,
            is_available: modifier.is_available,
            display_order: modifier.display_order,
            created_at: modifier.created_at,
            updated_at: modifier.updated_at,
        }
    }
}
