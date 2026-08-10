# 事务与一致性：本项目的事务机制详解

> 本文讲解本项目是如何用数据库事务保证「要么全部成功、要么全部回滚」的。
> 以"创建任务 + 附带首条笔记"（`POST /api/tasks/with-note`）为例，逐层拆解：
> 端口定义 → 应用层编排 → sqlx 实现 → SQLite 底层 → 错误路径 → 测试验证。

---

## 1. 为什么需要事务

业务里有一个跨两张表的用例：**创建任务并同时写入它的首条笔记**。

如果没有事务，可能出现「只插入了任务、笔记没插上」的孤儿数据。事务保证这两个写操作
**原子**：要么都成功（`COMMIT`），要么都失败回滚（`ROLLBACK`），数据库永远不会处于
"一半"的状态。

事务发生在**应用层用例**里，但应用层不直接写 SQL——它通过一个**南向端口**
（`TransactionManager`）向基础设施层要一个事务上下文，然后在事务里通过上下文调用仓储。

---

## 2. 架构设计：事务作为"南向端口"

菱形架构下，事务不是一条 SQL 命令，而是一个**领域层定义的端口（trait）**，
由 `south/`（南向网关）实现、`application/`（应用层）编排使用：

| 层 | 文件 | 职责 |
|----|------|------|
| **domain**（定义端口） | `src-tauri/src/domain/uow.rs` | `TransactionContext` / `TransactionManager` 两个 trait；只声明"能开事务、能拿仓储、能提交" |
| **application**（编排） | `src-tauri/src/application/task_service.rs` | `create_task_with_note`：`begin → 写任务 → 写笔记 → commit`；对 `TransactionManager` 泛型，不依赖任何 sqlx 类型 |
| **south**（实现） | `src-tauri/src/south/db/uow.rs` | `SqlxTransactionManager` / `SqlxTransactionContext` / `TxTaskRepository` / `TxNoteRepository`：真正操作 sqlx 事务 |
| **north**（入口） | `src-tauri/src/north/handlers/tasks.rs` | HTTP 入口，只调用北向端口 `TaskUseCase`，与事务细节无关 |

### 领域层端口（`domain/uow.rs`）

```rust
/// 事务上下文：一个已开启、尚未提交的事务所绑定的仓储集合。
#[async_trait::async_trait]
pub trait TransactionContext {
    type TaskRepo: TaskRepository;     // 关联类型：事务内绑定的任务仓储
    type NoteRepo: NoteRepository;     // 关联类型：事务内绑定的笔记仓储

    fn tasks(&mut self) -> &mut Self::TaskRepo;   // 借出事务内任务仓储（唯一可变借用）
    fn notes(&mut self) -> &mut Self::NoteRepo;   // 借出事务内笔记仓储（唯一可变借用）
    async fn commit(self) -> Result<(), RepoError>; // 提交；消费 self，提交后上下文不可再用
}

/// 事务管理器：由 south 层实现，应用层通过它开启新事务。
#[async_trait::async_trait]
pub trait TransactionManager: Send + Sync + 'static {
    type Context: TransactionContext + Send;  // 具体上下文类型（无 Box<dyn>、无虚表）
    async fn begin(&self) -> Result<Self::Context, RepoError>;
}
```

关键点（对应参考示例的生产级设计）：

- **依赖倒置**：`domain` 只声明接口，不 import 任何 sqlx / SQLite 代码；具体实现由
  `south` 提供、在组合根（`lib.rs`）注入。
- **关联类型 + 具体类型返回**：`begin()` 返回 `Self::Context`（具体类型），不像旧版
  返回 `Box<dyn UnitOfWork>`——无装箱、无虚表派发，IDE 可补全、借用检查在编译期生效。
- **`commit(self)` 消费上下文**：结构上杜绝"提交后再操作事务"——上下文都被拿走了，
  编译器不允许你再调 `ctx.tasks()`。
- **仓储不持有连接池、不自行开事务**：`TxTaskRepository` 只保存对共享事务的引用，
  事务边界完全由应用层用例控制（类似 Spring `@Transactional` 的"边界在上层"原则）。
- **`&mut` 借出仓储**：`ctx.tasks()` / `ctx.notes()` 返回 `&mut`，同一时刻只能借出
  一个，防止同一事务被两个仓储交错写入。

---

## 3. 完整调用链（从 HTTP 到 SQLite）

```
前端按 POST /api/tasks/with-note
   │
   ▼
north/handlers/tasks.rs :: create_task_with_note(state, JsonBody(payload))
   │  state.tasks.create_task_with_note(dto)
   ▼
application/task_service.rs :: create_task_with_note
   │  let mut ctx = self.tx_manager.begin().await?;  // ① 开启事务（从连接池借一条连接）
   │  let task = Task::new(&dto.title)?;             // ② 校验标题 → 构造任务实体
   │  ctx.tasks().insert(&task).await?;              // ③ 事务内 INSERT tasks
   │  let note = Note::new(Some(task.id()), &dto.content)?; // ④ 校验内容
   │  ctx.notes().insert(&note).await?;              // ⑤ 事务内 INSERT notes
   │  ctx.commit().await?;                           // ⑥ 全部成功 → COMMIT
   ▼
south/db/uow.rs :: SqlxTransactionManager::begin    →  pool.begin()  (BEGIN)
         SqlxTransactionContext::commit             →  tx.commit()   (COMMIT)
   ▼
SQLite (WAL)  data.db
```

### 应用层用例（`application/task_service.rs`）

```rust
pub struct TaskService<M: TransactionManager> {   // 对事务管理器泛型，不 import 任何 south 类型
    repo: Arc<dyn TaskRepository>,
    tx_manager: Arc<M>,
}

#[async_trait::async_trait]
impl<M: TransactionManager> TaskUseCase for TaskService<M> {
    async fn create_task_with_note(
        &self,
        dto: CreateTaskWithNoteDto,
    ) -> Result<Task, ServiceError> {
        let mut ctx = self.tx_manager.begin().await?;        // 开事务
        let task = Task::new(&dto.title)?;                   // 校验标题（不变量在实体里）
        ctx.tasks().insert(&task).await?;                    // 写任务
        let note = Note::new(Some(task.id().to_owned()), &dto.content)?; // 校验笔记
        ctx.notes().insert(&note).await?;                    // 写笔记
        ctx.commit().await?;                                 // 全部成功 → 提交
        Ok(task)
    }
}
```

注意 `?` 运算符：**任一步返回 `Err`（校验失败 / 写入失败），函数立即提前返回，
`ctx` 被 drop，事务自动回滚**。整个用例只写了 7 行，事务的打开/提交/回滚全部被
封装，业务代码不用手工 catch。

---

## 4. south 实现：`SqlxTransactionContext` 内部设计

### 4.1 整个事务只占"一个"共享句柄

普通读写各用连接池里的一条连接；事务则必须把**多个写操作钉在同一条连接上的同一个
事务里**。实现的关键是一个共享句柄：

```rust
type SharedTx = Arc<Mutex<Option<Transaction<'static, Sqlite>>>>;   // tokio::sync::Mutex
```

这是为了保证四个约束：

| 设计 | 解决的问题 |
|------|-----------|
| `Transaction` | sqlx 的连接级事务对象；事务内所有 SQL 都走它 |
| `Option` | `commit` 时用 `take()` 把事务"拿走"；之后任何事务内操作得到 `RepoError`（防提交后再写） |
| `Arc` | 任务仓储、笔记仓储、上下文**共享同一份**事务，而不是各开各的 |
| `Mutex` | 仓储方法需要 `&mut Transaction`（独占写），用锁保证同一时刻只有一个操作在事务上进行；`tokio::sync::Mutex` 的 guard 是 `Send`，可安全跨 `.await` 持有 |

事务内仓储拿到锁后再取出底层连接执行 SQL：

```rust
// TxTaskRepository::insert
let mut guard = self.shared.lock().await;                    // ① 拿锁（tokio Mutex）
let tx = guard.as_mut().ok_or_else(tx_ended)?;               // ② 取出事务，None → RepoError
super::task_repo::insert_task(&mut **tx, task).await         // ③ 执行 SQL
```

注意 **sqlx 0.8 中 `&mut Transaction` 不实现 `Executor`**（该 impl 在 sqlx 源码中
被注释掉，见 `sqlx-core/src/transaction.rs`），所以要解引用到底层连接：
`&mut **tx`（即 `&mut SqliteConnection`），后者实现 `Executor`。

为什么不用旧版的 `UnsafeCell`：它虽然省了锁，但依赖"单协程串行"这一运行时约定，
需要手写 `unsafe impl Send/Sync`，并发下不健全；`Mutex` 是标准、健全的原语，事务内
本来无争锁场景，性能差异可忽略。

为什么没有 `transmute`：sqlx 的 `Pool::begin()` 直接返回
`Transaction<'static, Sqlite>`（`sqlx-core/src/pool/mod.rs`），无需任何生命周期作弊。

### 4.2 SQL 复用：`Executor` 泛型助手

普通仓储用 `&Pool`（autocommit，每条语句独立提交），事务仓储用 `&mut SqliteConnection`。
为了让两侧 SQL 完全一致，SQL 被封装成接受**任意 `Executor`** 的 `pub(crate)` 函数：

```rust
// south/db/task_repo.rs：同一个函数，两种调用方式都能用
pub(crate) async fn insert_task<'e, E>(executor: E, task: &Task) -> Result<(), RepoError>
where
    E: Executor<'e, Database = Sqlite>,
{
    sqlx::query("INSERT INTO tasks (id, title, completed, created_at) VALUES (?, ?, ?, ?)")
        .bind(task.id()).bind(task.title())
        .bind(task.completed()).bind(task.created_at())
        .execute(executor)          // executor 是 &Pool 或 &mut SqliteConnection 都成立
        .await?;
    Ok(())
}
```

- 普通仓储：`SqlxTaskRepository::insert` → `insert_task(&self.pool, task)`
- 事务仓储：`TxTaskRepository::insert` → `insert_task(&mut **tx, task)`

好处：**同一份 SQL 两处即用，绝无"普通路径成功、事务路径失败"的差异**。

### 4.3 提交与"兜底回滚"

```rust
async fn commit(self) -> Result<(), RepoError> {
    let tx = self.shared.commit().await?;   // take() 取走事务（此后容器为 None）
    tx.commit().await?;                     // COMMIT → 事务内写入才对外可见
    Ok(())
}

// SharedTx::commit
async fn commit(self) -> Result<Transaction<'static, Sqlite>, RepoError> {
    let mut guard = self.tx.lock().await;
    guard.take().ok_or_else(|| RepoError::wrap("事务已提交，不能重复提交"))
}
```

**兜底保证**（非常重要）：sqlx 的 `Transaction` 在 `Drop` 时决定是否自动回滚——
只要 `commit()` 没有被调用，`Transaction` 被局部变量 `ctx` 释放时就会自动执行 `ROLLBACK`。

因此即使应用层**忘记调 `commit`** 或 **中途 `return`**，也不会残留半个事务的写入。
这是 SQL 层面之外的第二重保险。

---

## 5. 错误路径逐一分析

| 场景 | 走向 | 结果 |
|------|------|------|
| 全部步骤成功，按序 `commit` | `COMMIT` 落盘 | 任务 + 笔记都可见（唯一预期路径） |
| 笔记内容非法（`Note::new` 返回 `Err`） | `?` 提前返回，`ctx` 被 drop | 任务也被回滚，无孤儿数据 |
| 笔记 INSERT 违反外键（task_id 不存在） | `insert_note` 返回 `Err` → `?` 返回 | `ctx` drop → ROLLBACK |
| 插完任务后某个 `?` 抛出 Error | 同上 | 同上 |
| 忘记 `commit()` 就返回 | 局部变量 `ctx` 被 drop | sqlx `Drop` 自动回滚 |
| commit 成功后复用事务仓储 | 结构上不可能：`commit(self)` 已消费上下文，编译器阻止 | 防御性 `RepoError`（"事务已结束…"）兜底 |
| 重复调用 `commit()` | `take()` 已是 `None` → `RepoError` | 二次提交直接报错 |

**一致性（Consistency）的双重来源**：

1. **数据库约束**：`notes.task_id REFERENCES tasks(id) ON DELETE CASCADE`
   （`migrations/002_notes.sql`），外键约束由 SQLite 保证；
2. **领域不变量**：`Task::new` / `Note::new` 在校验标题/内容后再进事务，业务校验
   不外流到 SQL。

---

## 6. 底层与配置：SQLite 的几个保证

`src-tauri/src/south/db/mod.rs` 的连接池配置（`init_pool`）：

```rust
SqliteConnectOptions::from_str(&config.database_url)?
    .create_if_missing(true)
    .journal_mode(SqliteJournalMode::Wal)       // WAL：读写并发、崩溃恢复
    .synchronous(SqliteSynchronous::Normal)     // 提交性能与安全的折中
    .busy_timeout(Duration::from_secs(5));      // 锁等待，避免立即死锁报错
SqlitePoolOptions::new().max_connections(config.db_max_connections)  // 默认 5
```

对应事务相关的四个特性：

| SQLite 机制 | 本项目的设置 | 对事务的意义 |
|-------------|--------------|--------------|
| WAL 日志模式 | `journal_mode = Wal` | 写事务用一个 `-wal` 文件；崩溃后能重建，读并发更好 |
| `synchronous = Normal` | 默认 | 与 WAL 搭配默认，崩溃时最多丢最近一个事务但**不会损坏/不会出现部分提交** |
| `busy_timeout = 5s` | 有 | 一行被别的连接锁住时先等而不是立刻报错 |
| 外键约束 | sqlx 对 SQLite 默认开启 `PRAGMA foreign_keys = ON` | 事务拒绝写入违反 FK 的行（这是回滚测试的原动力） |

**隔离性（Isolation）**：SQLite 本身是单写者数据库，事务默认是
`SERIALIZABLE`。事务内写入对其他连接**不可见**，直到 `commit()` 提交。本项目
事务只使用单条连接（`Mutex` 串行），没有并发写事务重叠，隔离性天然满足。

**一个事务就是一条连接**：`max_connections = 5` 意味着同时最多 5 个事务在跑，
超过的会等待连接池释放。所以事务要**短平快**，不要在事务里做耗时的外部调用。

---

## 7. 测试验证（`south/db/uow.rs` 自带）

项目里已经有三个针对事务的回归测试，直接证明"事务是真的"：

```rust
// ① 提交成功 → 任务与笔记都可见
#[tokio::test]
async fn commit_persists_task_and_note() {
    let mut ctx = manager.begin().await.expect("begin failed");
    ctx.tasks().insert(&task).await.expect("insert task");
    ctx.notes().insert(&note).await.expect("insert note");
    ctx.commit().await.expect("commit failed");
    // 然后 pool：SELECT COUNT(*) FROM tasks / FROM notes 均为 1
}

// ② 中途失败 → 不提交 → 第一步写入一并回滚
#[tokio::test]
async fn mid_transaction_failure_rolls_back() {
    let mut ctx = manager.begin().await.expect("begin failed");
    ctx.tasks().insert(&task).await.expect("insert task");
    // 第二步写入不存在的 task_id → 外键拦截，返回 Err
    assert!(ctx.notes().insert(&bad_note).await.is_err(), "外键应拦截无效笔记");
    drop(ctx); // 不提交，直接丢弃 → 自动回滚
    // 断言 tasks 计数为 0：任务没有残留
}

// ③ begin() 返回具体上下文，无 Box<dyn>；commit 消费上下文
#[tokio::test]
async fn begin_returns_concrete_context() { ... }
```

跑法：

```bash
cd src-tauri && cargo test uow
```

注意：这些测试都使用唯一临时数据库文件（`axum_uow_test_<uuid>.db`），跑完即删，
不会污染项目内的 `data.db`。

---

## 8. 局限与使用约定

- **事务只给真正需要原子性的用例**：单次写（如 `POST /api/tasks`、`PUT`）不走
  `begin`，直接用普通仓储（每条语句一个隐式短事务），避免不必要的开销。
- **当前只有 `create_task_with_note` 一个事务用例**。未来"删除任务并清理其全部笔记"
  （可通过 `ON DELETE CASCADE` 完成）或"批量导入"等场景直接在照葫芦画瓢端口即可：
  在领域层加用例方法，组合根注入新的 `Arc<dyn ...>` 或复用现有 `SqlxTransactionManager`。
- **事务内是串行的**：`Mutex` 保证同一时刻只有一个 SQL 在事务上执行；千万不要在
  事务内并发 `spawn` 两个事务内仓储调用（会死锁/逻辑错）。
- **事务不能跨请求**：`ctx` 是局部变量，HTTP 请求结束即被 drop → 自动回滚。
- **前端还没有接入 `with-note` 接口**：后端 `POST /api/tasks/with-note` 已就绪，
  前端 store / 仓储尚未封用它，未来新增"创建任务时直接填首条笔记"的 UI 时对接即可。
- **隔离级别**：SQLite 事务即 `SERIALIZABLE`（单写者），没有读已提交/可重复读的级别；
  由于本应用是本地单用户桌面程序，几乎不存在并发写冲突。

---

## 9. 代码速查

| 你要找的东西 | 位置 |
|--------------|------|
| 事务端口定义（`TransactionContext` / `TransactionManager`） | `src-tauri/src/domain/uow.rs` |
| sqlx 事务实现（`SqlxTransactionManager` / `SqlxTransactionContext` / `SharedTx` / `TxTaskRepository` / `TxNoteRepository`） | `src-tauri/src/south/db/uow.rs` |
| 事务用例编排（对 `TransactionManager` 泛型） | `src-tauri/src/application/task_service.rs:56` |
| HTTP 入口 | `src-tauri/src/north/handlers/tasks.rs` |
| 组合根注入（`SqlxTransactionManager::new(pool)`） | `src-tauri/src/lib.rs:58` |
| SQL 复用助手（`Executor` 泛型） | `src-tauri/src/south/db/task_repo.rs` / `note_repo.rs` |
| 连接池 / WAL / 外键配置 | `src-tauri/src/south/db/mod.rs:28` |
| 表结构与外键 / 级联删除 | `src-tauri/migrations/002_notes.sql` |
| 事务回归测试 | `src-tauri/src/south/db/uow.rs` 的 `mod tests` |
