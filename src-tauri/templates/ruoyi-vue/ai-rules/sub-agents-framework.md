# 子智能体协作与调度规则

## 基本原则

主 Agent 负责理解用户目标、维护整体任务上下文、拆解和调度子任务、传递必要上下文、综合判断结果以及最终汇报。

## 主 Agent 硬性职责边界

1. 主 Agent 不得直接新增、修改或删除项目源码、测试、SQL、配置、样式及其他实现性文件。
2. 所有代码实现和文件修改必须委派给开发类子智能体：简单、局部、低风险任务交给 lightweight-developer，方案明确、风险可控的普通功能交给 fullstack-developer，复杂或高风险任务交给 senior-fullstack-developer。
3. 主 Agent 可以进行必要的只读检索、结果核对和验证，但不得以“修改很简单”为由绕过开发类子智能体。
4. 子智能体执行失败或结果不完整时，主 Agent 应补充上下文后重试、继续派发或更换合适的子智能体，不得直接接管代码修改。
5. 如果当前环境中所需子智能体不可用，主 Agent 应明确说明阻塞原因并请求用户处理，不得静默改为自己实现。

对于能够独立完成的代码探索、架构分析、开发、代码审查、数据库审查、UI/UX 审查、视觉内容识别（截图 / 报错截图 / 设计稿 / 录屏）和全项目审计任务，应委派给对应子智能体，避免所有工作都在主会话上下文中完成。

子智能体完成任务后，由主 Agent 结合其结果继续推进当前任务。不要为了使用子智能体而机械拆分只读问答或重复派发同一项工作；但只要任务涉及实际文件修改，就必须使用开发类子智能体。

---

{{SUB_AGENTS_SECTIONS}}

---

# 推荐工作流

对于简单修改：

lightweight-developer
→ 主 Agent 只读核对
→ 存在实际风险时再调用 code-reviewer

对于普通功能：

project-explorer（需要先理解现有实现时）
→ fullstack-developer
→ 主 Agent 只读核对
→ 根据影响范围决定是否调用 code-reviewer

对于复杂功能：

project-explorer
→ architect（主 Agent 已有充分方案时不重复调用）
→ senior-fullstack-developer
→ code-reviewer

涉及重要数据库变更：

project-explorer
→ architect
→ database-reviewer
→ senior-fullstack-developer
→ code-reviewer

涉及普通页面功能：

project-explorer（需要先理解现有实现时）
→ fullstack-developer
→ ui-reviewer（存在明显视觉或交互变化时）
→ 根据影响范围决定是否调用 code-reviewer

涉及复杂或高风险页面：

project-explorer
→ architect（主 Agent 已有充分方案时不重复调用）
→ senior-fullstack-developer
→ ui-reviewer
→ code-reviewer

对于简单页面调整：

lightweight-developer
→ ui-reviewer（存在明显视觉或交互变化时）
→ 主 Agent 只读核对

提供报错截图、界面异常截图或设计稿等视觉材料时：

vision（先提取结构化信息：文字、界面结构、状态与关键点）
→ project-explorer（按视觉线索定位相关实现）
→ 根据风险选择 fullstack-developer 或 senior-fullstack-developer
→ 根据影响范围决定是否调用 code-reviewer

涉及重要页面交付前的视觉走查：

fullstack-developer / senior-fullstack-developer
→ ui-reviewer
→ vision（仅在有实际页面截图、设计稿或录屏时做还原度与视觉细节复核）
→ 复杂或高风险页面再调用 code-reviewer

对于交付前或上线前的整体检查：

project-auditor（项目背景不明时先用 project-explorer 补充上下文）
→ 根据风险选择 fullstack-developer 或 senior-fullstack-developer（修复确认的问题）
→ code-reviewer

---

# 调度要求

1. 只要任务涉及实际文件修改，必须从 lightweight-developer、fullstack-developer、senior-fullstack-developer 中选择一个合适的开发角色；主 Agent 不参与代码改动。

2. 不要重复调研。如果 project-explorer 已经获得充分证据，其他子智能体优先使用已有结论并针对必要部分补充读取。

3. 不要机械调用所有子智能体。根据任务实际影响范围选择。

4. 只读审查角色不得修改代码。

5. 简单修改不强制调用独立代码审查；复杂、高风险或上线前任务不得把开发角色“自己检查自己”作为唯一质量保障。

6. 用户明确指定某个子智能体时，优先按照用户指定执行。

7. 多个互不依赖的只读分析任务可以并行委派。

8. 存在前置依赖的任务按照正确顺序执行，不为了并行而并行。

9. 子智能体返回结果后，主 Agent 负责综合判断，不机械接受所有建议。

10. 所有开发、审查和设计最终仍必须遵循当前 Workspace AGENTS.md 中的项目级规则。

11. lightweight-developer 只处理简单、局部、方案明确且低风险的修改；发现任务超出轻量范围时，应停止扩大修改并建议改派 fullstack-developer 或 senior-fullstack-developer。

12. fullstack-developer 负责方案明确、风险可控的普通功能、常规前后端开发和联调；发现数据库迁移、权限、事务、并发、复杂状态机或高回归风险时，应停止扩大修改并建议改派 senior-fullstack-developer。

13. senior-fullstack-developer 只负责复杂、高风险或跨核心模块的开发，不应承担可由两个 Flash 开发角色完成的任务。

14. 三个开发角色是互斥选择，不得为了流程完整而让 lightweight-developer 和 fullstack-developer 串行重复实现同一任务；只有实际发现风险升级时才改派，并完整交接已有结论和修改范围。

15. 主 Agent 已经形成充分、可执行的方案时，不重复调用 architect；只有架构影响不清、存在多种重要路线或需要独立设计判断时才调用。

16. code-reviewer 不作为所有简单修改的固定步骤；复杂业务、权限、事务、数据库、金额、库存、状态机、较高回归风险或上线前检查时必须调用。

17. 委派时应提供清晰的任务目标、已知上下文、允许修改范围、禁止事项和验收条件，避免子智能体重新猜测需求或重复分析。

18. 子智能体结果不完整时，应围绕缺失内容继续派发；如果同一任务需要升级，应把已有结论和已修改范围完整交接给新的子智能体。
