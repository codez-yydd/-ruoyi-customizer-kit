# 子智能体协作与调度规则

## 基本原则

主 Agent 负责理解用户目标、维护整体任务上下文、协调子任务和最终汇总。

对于能够独立完成的代码探索、架构分析、开发、代码审查、数据库审查、UI/UX 审查和全项目审计任务，应优先考虑委派给对应子智能体，避免所有工作都由主 Agent 在同一上下文中完成。

子智能体完成任务后，由主 Agent 结合其结果继续推进当前任务。

不要为了使用子智能体而机械拆分简单任务。
对于非常简单、局部、明确且无需独立上下文的修改，可以直接处理。

---

{{SUB_AGENTS_SECTIONS}}

---

# 推荐工作流

对于简单修改：

主 Agent / fullstack-developer
→ 必要验证

对于普通功能：

Explore
→ fullstack-developer
→ code-reviewer

对于复杂功能：

Explore
→ architect
→ fullstack-developer
→ code-reviewer

涉及重要数据库变更：

Explore
→ architect
→ database-reviewer
→ fullstack-developer
→ code-reviewer

涉及重要页面：

Explore
→ architect（复杂页面时）
→ fullstack-developer
→ ui-reviewer
→ code-reviewer

对于交付前或上线前的整体检查：

project-auditor（项目背景不明时先用 Explore 补充上下文）
→ fullstack-developer（修复确认的问题）
→ code-reviewer

---

# 调度要求

1. 子智能体适合独立完成的工作，优先委派，减少主会话无意义上下文增长。

2. 不要重复调研。如果 Explore 已经获得充分证据，其他子智能体优先使用已有结论并针对必要部分补充读取。

3. 不要机械调用所有子智能体。根据任务实际影响范围选择。

4. 只读审查角色不得修改代码。

5. 开发角色完成修改后，不应把“自己检查自己”作为大型任务唯一的质量保障。

6. 用户明确指定某个子智能体时，优先按照用户指定执行。

7. 多个互不依赖的只读分析任务可以并行委派。

8. 存在前置依赖的任务按照正确顺序执行，不为了并行而并行。

9. 子智能体返回结果后，主 Agent 负责综合判断，不机械接受所有建议。

10. 所有开发、审查和设计最终仍必须遵循当前 Workspace AGENTS.md 中的项目级规则。
