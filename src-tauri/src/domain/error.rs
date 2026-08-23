//! 领域不变量错误。
//!
//! 实体构造/变更方法在校验不通过时返回这些错误。它们只描述"业务规则被违反"，
//! 不携带 HTTP 概念；HTTP 状态码由表现层在映射时决定。

use thiserror::Error;

/// 领域规则错误，覆盖任务/笔记的全部不变量。
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum DomainError {
    #[error("任务标题不能为空。")]
    TaskTitleEmpty,
    #[error("任务标题必须是 1 到 120 个字符。")]
    TaskTitleTooLong,
    #[error("笔记内容不能为空。")]
    NoteContentEmpty,
    #[error("笔记内容必须是 1 到 5000 个字符。")]
    NoteContentTooLong,
    #[error("分类名称不能为空。")]
    CategoryNameEmpty,
    #[error("分类名称必须是 1 到 40 个字符。")]
    CategoryNameTooLong,
    #[error("分类不存在或已被删除。")]
    CategoryNotFound,
    #[error("至少提供一个需要更新的字段。")]
    EmptyUpdate,
    #[error("任务不存在或已被删除。")]
    TaskNotFound,
    #[error("笔记不存在或已被删除。")]
    NoteNotFound,
}
