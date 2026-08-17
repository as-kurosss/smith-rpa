//! Общий модуль: разрешение UI-элемента по ключу контекста или селектору.

use crate::element::SafeUIElement;
use crate::selector::ElementSelector;
use smith_core::{ExecutionContext, ToolError};

/// Поля селектора, общие для всех инструментов, работающих с элементами.
pub struct SelectorFields<'a> {
    pub name: Option<&'a str>,
    pub automation_id: Option<&'a str>,
    pub control_type: Option<&'a str>,
    pub class_name: Option<&'a str>,
}

/// Разрешает UI-элемент из конфигурации шага.
///
/// Порядок поиска:
/// 1. Ключ `element_key` в `ExecutionContext`
/// 2. Построение `ElementSelector` по селекторным полям и поиск с десктопа
/// 3. Если ничего не задано — `Ok(None)`
///
/// # Errors
///
/// Возвращает `ToolError` если ключ не найден или элемент не обнаружен.
pub async fn resolve_element(
    element_key: Option<&str>,
    selector: SelectorFields<'_>,
    ctx: &ExecutionContext,
) -> Result<Option<SafeUIElement>, ToolError> {
    // 1. Пробуем достать элемент из контекста
    if let Some(key) = element_key {
        let value = ctx.get(key).ok_or_else(|| {
            ToolError::invalid_input(
                format!("Key '{key}' not found in context"),
                Some("element_key".into()),
                None,
            )
        })?;
        return value
            .try_as_custom::<SafeUIElement>()
            .map(|e| Some(e.clone()));
    }

    // 2. Пробуем найти элемент по селекторным полям
    let has_selector = selector.name.is_some()
        || selector.automation_id.is_some()
        || selector.control_type.is_some()
        || selector.class_name.is_some();

    if !has_selector {
        return Ok(None);
    }

    let mut elem_selector = ElementSelector::new();
    if let Some(name) = selector.name {
        elem_selector = elem_selector.name(name);
    }
    if let Some(aid) = selector.automation_id {
        elem_selector = elem_selector.automation_id(aid);
    }
    if let Some(ct) = selector.control_type {
        elem_selector = elem_selector.control_type(ct);
    }
    if let Some(cn) = selector.class_name {
        elem_selector = elem_selector.class_name(cn);
    }

    let safe_element = tokio::task::spawn_blocking(move || {
        elem_selector.find_from_desktop().map(SafeUIElement::new)
    })
    .await
    .map_err(|e| ToolError::platform_error("Find element blocking task failed", e, None))?
    .map_err(|_e| ToolError::element_not_found("No element found matching selector", None))?;

    Ok(Some(safe_element))
}
