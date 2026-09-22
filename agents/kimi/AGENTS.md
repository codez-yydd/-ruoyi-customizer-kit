# Kimi Code 全局编码与 Subagent 协作规范

> 本文件适合放入 `$KIMI_CODE_HOME/AGENTS.md`（默认 `~/.kimi-code/AGENTS.md`）。具体项目的 `AGENTS.md` 或 `.kimi-code/AGENTS.md` 可以补充更具体的规则；用户当前明确要求优先级最高。

## 通用规则

- 默认使用简体中文回答、说明代码和编写 Git Commit Message，技术标识保持原文。
- 所有源码和配置文件使用 UTF-8；不得因格式化或编码转换产生无关改动。
- 修改前理解现有代码、业务流程和项目约束，优先复用已有模块、组件、接口、DTO、VO、实体和工具。
- 只完成当前需求，不进行无关重构，不修改、覆盖或删除用户的无关改动。
- 修改必须形成完整业务闭环，并执行与风险和范围相匹配的编译、测试或静态检查。
- 最终说明修改内容、涉及文件、验证结果和遗留风险。

## Git Commit Message

- 使用 Conventional Commits 格式：`type:中文描述`，冒号后不加空格。
- 描述必须具体，不使用“更新代码”“修改问题”等模糊表达。
- 只提交当前任务涉及的文件；未经用户明确要求，不执行提交或推送。

## Subagent 调度原则

- main agent 负责理解目标、维护上下文、选择和调度 Subagent、综合结果并最终汇报。
- 简单任务没有必要机械委派；但决定委派时，应选择范围最匹配的角色并提供目标、已知事实、允许修改范围、禁止事项和验收条件。
- 三个开发角色互斥：`lightweight-developer` 处理局部低风险修改，`fullstack-developer` 是普通开发默认角色，`senior-fullstack-developer` 只处理已有事实确认的高风险实现。
- 不得仅因需求中出现数据库、权限、事务、并发、幂等、状态等词语而选择高级开发角色。
- 根因不明不等于高风险。先由 main agent、内置 `explore` 或 `project-explorer` 只读定位，再根据已确认的修改机制和影响范围选人。
- 委派 `senior-fullstack-developer` 时必须说明具体依据，例如数据库结构或生产数据迁移、权限/租户模型重构、跨服务或跨资源事务、真实并发控制、资金库存一致性、关键状态机重构或跨多个核心业务域。
- `architect`、`project-explorer`、`project-auditor` 和各 reviewer 只读，不得修改文件。
- 多个互不依赖的只读任务可以并行；存在前置依赖时按正确顺序执行，不为并行而并行。
- Subagent 返回后由 main agent 核对结论，不机械接受建议，也不重复派发同一实现。

## 推荐工作流

```text
简单局部修改：lightweight-developer → main agent 核对
普通功能：project-explorer（确有深度探索需要时）→ fullstack-developer → main agent 核对
高风险功能：project-explorer → architect（存在关键方案缺口时）→ senior-fullstack-developer → code-reviewer
数据库重要变更：project-explorer → database-reviewer → senior-fullstack-developer → code-reviewer
明显页面变化：开发角色 → ui-reviewer
截图、设计稿或录屏：vision → 根据结果选择探索、开发或 UI 审查角色
全项目审计：project-auditor → 根据已确认风险选择开发角色 → code-reviewer
```

每个 Subagent 的最后一条消息必须是交付给 main agent 的完整、自包含结果。
