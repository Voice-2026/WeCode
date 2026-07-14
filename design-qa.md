# Agent 关注页面设计 QA

- 验收日期：2026-07-12
- 验收对象：本地 WeCode 隔离 debug 实例
- 验收范围：顶部“关注” Tab、概览卡片、空态、最小可见宽度和真实事件跳转
- 当前结论：部分通过

## 已通过

- 顶部独立“关注” Tab 可正常打开并显示选中态。
- “待处理 / 已完成 / 进行中”三类概览卡片层级清晰。
- Feed 无数据时显示“暂无 Agent 动态”空态，文案和布局正常。
- 尝试缩窄窗口时，应用保持最小可见宽度；该宽度下工具栏、卡片和空态无截断。
- 页面内容使用工作区可用宽度，没有恢复为居中窄容器。
- 独立工具栏铃铛已移除，未读提示统一收敛到“关注” Tab。

## 待补验收

- 当前 Feed 的待处理、已完成和进行中均为 `0`，无法验证未读角标和“全部已读”操作。
- 新建独立 `Codex` 终端未产生可识别的 Agent 运行事件，Feed 仍为 `0/0/0`。
- 跨项目/worktree 点击跳转必须等真实 Agent 进入等待输入、完成或中断状态后复测。

## 自动验证

- `cargo check -p wecode`：通过。
- `cargo test -p wecode`：215/215 通过。
- `git diff --check`：通过。

## 后续准入

需要一次真实 Agent 状态变化，并完成以下验证后，才能将设计 QA 改为全量通过：

1. 未读角标数量正确。
2. 点击事件能切换到对应项目、worktree 和终端。
3. Agent 恢复工作后，旧的待处理事件能自动解决。

---

# 自动任务编辑器设计 QA

- 验收日期：2026-07-14
- source visual truth path：`design-qa-assets/automation-editor-option2/source.png`
- implementation screenshot path：`design-qa-assets/automation-editor-option2/implementation-main.jpeg`
- implementation dropdown screenshot path：`design-qa-assets/automation-editor-option2/implementation-dropdown.jpeg`
- implementation bottom screenshot path：`design-qa-assets/automation-editor-option2/implementation-bottom.jpeg`
- theme segmented screenshot path：`design-qa-assets/automation-editor-option2/implementation-theme-segmented.jpeg`
- advanced settings source path：`design-qa-assets/automation-editor-option2/source-advanced-lines.png`
- advanced settings screenshot path：`design-qa-assets/automation-editor-option2/implementation-advanced-static.jpeg`
- advanced settings comparison path：`design-qa-assets/automation-editor-option2/comparison-advanced-static.png`
- toolbar source path：`design-qa-assets/automation-editor-option2/source-toolbar-group.png`
- toolbar first-pass screenshot path：`design-qa-assets/automation-editor-option2/implementation-toolbar-right.jpeg`
- toolbar vertical screenshot path：`design-qa-assets/automation-editor-option2/implementation-toolbar-vertical.jpeg`
- toolbar panel-open screenshot path：`design-qa-assets/automation-editor-option2/implementation-toolbar-vertical-panel.jpeg`
- toolbar vertical comparison path：`design-qa-assets/automation-editor-option2/comparison-toolbar-vertical.png`
- card redesign source path：`design-qa-assets/automation-editor-option2/source-stepper-before-card.jpeg`
- card redesign screenshot path：`design-qa-assets/automation-editor-option2/implementation-card-editor.jpeg`
- card redesign bottom screenshot path：`design-qa-assets/automation-editor-option2/implementation-card-editor-bottom.jpeg`
- header-line fix screenshot path：`design-qa-assets/automation-editor-option2/implementation-card-editor-no-header-line.jpeg`
- card redesign comparison path：`design-qa-assets/automation-editor-option2/comparison-card-editor.png`
- list spacing screenshot path：`design-qa-assets/automation-editor-option2/implementation-list-card-spacing.jpeg`
- editor keeps list screenshot path：`design-qa-assets/automation-editor-option2/implementation-editor-keeps-list.jpeg`
- list switch screenshot path：`design-qa-assets/automation-editor-option2/implementation-list-switch.jpeg`
- editor keeps list comparison path：`design-qa-assets/automation-editor-option2/comparison-editor-keeps-list.png`
- normalized comparison path：`design-qa-assets/automation-editor-option2/comparison.png`
- viewport：源设计 1578 × 997；实现 1199 × 768，macOS 浅色主题
- state：编辑“每日变更检查”，Forge 固定工作树、Claude、新建会话、工作日 10:10、24 小时补跑
- final result: passed

## Full-view comparison evidence

- 源设计与实现截图已归一到相同的 789 × 499 画布，并在 `comparison.png` 中左右并排检查。
- 编辑器保持左对齐和 920px 受控阅读宽度，不横向铺满所有可用区域。
- “基本信息 / 运行环境 / 执行计划 / 任务内容 / 高级设置”统一为独立卡片；移除了左侧编号圆点、粉色进度线和步骤分隔线。
- 卡片统一使用主题背景、边框、圆角和内边距，完成状态保留在卡片标题右侧，不再占用单独步骤栏。
- 标题区使用“自动任务 / 编辑任务”和日程、项目、智能体摘要，移除了编辑态下重复的外层页面标题和“新建任务”按钮。
- 任务名称与项目为两列；运行环境保持工作区、智能体、会话一行；执行计划保持日程、时间、时区、补跑时限一行。
- 实现保留 WeCode 项目侧栏和顶部工具栏；编辑区依据最新反馈从步骤流切换为卡片层级，同时保持原有受控宽度。

## Focused region comparison evidence

- 工作区和日程 Select 使用 gpui-component 原生弹层，触发、对齐、选中态、Esc 关闭均正常。
- 高级设置常驻展开并纳入相同卡片体系，移除折叠箭头和独立区隔线，两项检查设置保持两列。
- 编辑任务时左侧 300px 任务列表继续显示，编辑表单只替换右侧详情区域；点击另一任务可退出编辑并切换详情。
- 任务列表卡片统一限制为 82px，高度较长的日程和状态信息使用单行截断；卡片之间保留 10px 明确间距。
- 1199px 窗口下右侧编辑表单自动切换为紧凑列数，工作区、智能体、会话和执行计划均未裁切或溢出。
- 工具图标从顶部移除，在工作区最右侧形成固定 44px 竖栏；原有顺序、图标和交互保持不变。
- 助手面板在竖栏左侧打开，竖栏不会随面板开关移动或消失。
- 底部截图确认任务提示词、高级设置、取消和保存修改均可通过正常滚动到达，没有遮挡或裁切。
- `comparison-card-editor.png` 完成旧步骤流与新卡片流的同尺寸并排检查；`comparison-advanced-static.png` 和 `comparison-toolbar-vertical.png` 保留此前高级设置与竖向工具栏的局部证据。

## Required fidelity surfaces

- Fonts and typography：沿用 WeCode 系统字体；页面标题、卡片标题、说明、标签和输入值层级清晰，没有异常换行或截断。
- Spacing and layout rhythm：页面左侧 40px 起始，编辑内容最大宽度 920px；卡片间距、卡片内边距和字段网格保持稳定。
- Colors and visual tokens：完成图标、分段控件和主按钮复用主题主色；卡片背景、边框、文字和禁用态复用现有主题 token。
- Image quality and asset fidelity：页面没有图片资产；完成状态、设置和展开图标来自现有图标库，没有手绘 SVG、字符图标或占位图。
- Copy and content：保留真实任务名称、项目、智能体、会话、日程、时区、补跑与提示词文案。
- Responsiveness：1199 × 768 下字段不重叠、不溢出；底部内容通过主滚动区到达，Select 浮层保持在可视区域内。
- Accessibility：字段继续使用 Form/Field 组件标签；Select 和 ButtonGroup 保留键盘焦点与方向键行为；完成状态使用现有完成图标，不改变输入控件语义。

## Findings

- 无遗留 P0/P1/P2 问题。
- P3：源设计没有 WeCode 固定侧栏和顶部工具栏，实现的首屏纵向空间更小，因此 768px 高度下需要滚动才能看到操作栏。
- 分段按钮选中项使用白底、主题粉描边和文字，未选项保持中性灰，与源设计一致。

## Comparison history

1. 第一轮发现 P1：编辑态同时显示外层“自动任务”栏和编辑器“自动任务 / 编辑任务”，标题重复且额外占用 58px。
2. 第一轮发现 P2：旧实现字段横向铺满工作区，缺少源设计的连续步骤和受控阅读宽度。
3. 修复：编辑态隐藏外层列表页标题和“新建任务”按钮，只保留编辑器标题与摘要。
4. 修复：表单外壳收敛到 920px，重构为四段编号步骤，补充完成图标和连续粉色引导线。
5. 修复：高级设置最初改为独立折叠行；任务提示词高度缩短；底部操作区与编辑内容对齐。
6. 复验：主视图、Select 弹层、高级设置展开、滚动和底部操作区均未发现新的 P0/P1/P2 问题。
7. 复验：工作区与会话分段按钮仅选中项使用主题粉描边和文字，未选项不再带主题色。
8. 第二轮发现 P2：高级设置下方同时存在容器底线与操作栏顶线，形成重复横线；折叠入口增加了不必要操作。
9. 修复：高级设置改为常驻展开，移除折叠状态、箭头和点击区域；删除下方两条线，仅保留上方区隔线。
10. 第一版误把助手工具按钮继续横向放在标题栏最右端，不符合“右侧增加一列”的真实意图。
11. 修复：从顶部移除整组按钮，在工作区右侧增加固定 44px 竖栏，按 Server、AI、SSH、DB、Files、Git 自上而下排列。
12. 复验：点击 AI 图标后面板在竖栏左侧打开，再次点击关闭；竖栏位置、顺序和选中态保持稳定。
13. 局部并排图确认高级设置没有重复横线，工具图标已改为窗口右侧竖向排列；没有新的 P0/P1/P2 问题。
14. 根据最新反馈移除左侧步骤栏，将五个设置区统一为独立卡片，并保留右侧完成状态、原字段布局和保存逻辑。
15. `comparison-card-editor.png` 同尺寸并排确认：卡片边界清晰、间距一致，高级设置与其余区域视觉语言统一，没有新的重叠、裁切或溢出。
16. 移除页面标题区底边框，基本信息卡片上方不再出现多余横线；隔离版复验通过。
17. 编辑器从全宽覆盖改为仅占右侧详情区，左侧任务列表在编辑期间保持可见。
18. 任务列表卡片固定为 82px，长内容单行截断，并以 10px 外边距隔开相邻卡片。
19. 首次分栏复验发现 1199px 窗口仍使用宽屏四列，导致会话控件裁切；将响应式断点调整为 1480px 后改用紧凑布局，复验通过。
20. 点击左侧另一任务后正确退出编辑并打开对应详情；没有改变任务数据。

## Primary interactions tested

- 打开顶部“自动任务”。
- 选择现有任务并进入“编辑”。
- 展开日程 Select，检查弹层位置和选中态，并使用 Esc 关闭。
- 滚动到表单底部，确认高级设置无需操作即可显示。
- 点击右侧 AI 图标打开助手面板，再次点击关闭。
- 未点击“保存修改”，未改变真实任务数据。
- 原生 GPUI 应用无浏览器 console；运行期间未发现 panic/fatal。
- `cargo test -p wecode`：218/218 通过。
- `cargo build -p wecode`：通过。
- `git diff --check`：通过。
