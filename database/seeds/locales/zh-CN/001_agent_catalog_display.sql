-- SDKWork official agent catalog: zh-CN knowledge-space display.

-- Catalog revision: 1.3.0.

BEGIN;

SELECT set_config('app.current_tenant_id', '100001', true);

SELECT set_config('app.current_organization_id', '0', true);

UPDATE kb_space SET
  name = '全栈开发工程师知识库',
  description = '面向全栈软件工程师的范围说明、专业标准、模板与检查清单。主题：engineering、programmer、fullstack、web、api。',
  updated_at = TIMESTAMPTZ '2026-11-03T00:00:00+08:00'
WHERE id = 533000000000000001 AND tenant_id = 100001 AND organization_id = 0;

UPDATE kb_space SET
  name = '前端开发工程师知识库',
  description = '面向前端工程师的范围说明、专业标准、模板与检查清单。主题：engineering、programmer、frontend、react、accessibility。',
  updated_at = TIMESTAMPTZ '2026-11-03T00:00:00+08:00'
WHERE id = 533000000000000002 AND tenant_id = 100001 AND organization_id = 0;

UPDATE kb_space SET
  name = 'K12 数学老师知识库',
  description = '面向中小学数学教师的范围说明、专业标准、模板与检查清单。主题：education、k12、math、curriculum。',
  updated_at = TIMESTAMPTZ '2026-11-03T00:00:00+08:00'
WHERE id = 533000000000000003 AND tenant_id = 100001 AND organization_id = 0;

UPDATE kb_space SET
  name = '大学计算机科学导师知识库',
  description = '面向计算机科学课程导师的范围说明、专业标准、模板与检查清单。主题：education、university、computer-science、algorithms。',
  updated_at = TIMESTAMPTZ '2026-11-03T00:00:00+08:00'
WHERE id = 533000000000000004 AND tenant_id = 100001 AND organization_id = 0;

UPDATE kb_space SET
  name = '全科医学顾问知识库',
  description = '面向全科医学顾问的范围说明、专业标准、模板与检查清单。主题：healthcare、doctor、general-practice、triage、health。',
  updated_at = TIMESTAMPTZ '2026-11-03T00:00:00+08:00'
WHERE id = 533000000000000005 AND tenant_id = 100001 AND organization_id = 0;

UPDATE kb_space SET
  name = '民商事律师知识库',
  description = '面向中国民商事法律顾问的范围说明、专业标准、模板与检查清单。主题：legal、lawyer、civil、contract、dispute。',
  updated_at = TIMESTAMPTZ '2026-11-03T00:00:00+08:00'
WHERE id = 533000000000000006 AND tenant_id = 100001 AND organization_id = 0;

UPDATE kb_space SET
  name = '旅行规划师知识库',
  description = '面向旅行规划师的范围说明、专业标准、模板与检查清单。主题：travel、itinerary、budget、planning。',
  updated_at = TIMESTAMPTZ '2026-11-03T00:00:00+08:00'
WHERE id = 533000000000000007 AND tenant_id = 100001 AND organization_id = 0;

UPDATE kb_space SET
  name = '电商运营顾问知识库',
  description = '面向电商运营顾问的范围说明、专业标准、模板与检查清单。主题：ecommerce、operations、growth、conversion。',
  updated_at = TIMESTAMPTZ '2026-11-03T00:00:00+08:00'
WHERE id = 533000000000000008 AND tenant_id = 100001 AND organization_id = 0;

UPDATE kb_space SET
  name = '购房顾问知识库',
  description = '面向购房顾问的范围说明、专业标准、模板与检查清单。主题：realestate、buyer、due-diligence、negotiation。',
  updated_at = TIMESTAMPTZ '2026-11-03T00:00:00+08:00'
WHERE id = 533000000000000009 AND tenant_id = 100001 AND organization_id = 0;

UPDATE kb_space SET
  name = 'K12 语文老师知识库',
  description = '面向中小学语文教师的范围说明、专业标准、模板与检查清单。主题：education、k12、chinese、reading、writing。',
  updated_at = TIMESTAMPTZ '2026-11-03T00:00:00+08:00'
WHERE id = 533000000000000010 AND tenant_id = 100001 AND organization_id = 0;

UPDATE kb_space SET
  name = 'K12 英语老师知识库',
  description = '面向中小学英语教师的范围说明、专业标准、模板与检查清单。主题：education、k12、english、listening、writing。',
  updated_at = TIMESTAMPTZ '2026-11-03T00:00:00+08:00'
WHERE id = 533000000000000011 AND tenant_id = 100001 AND organization_id = 0;

UPDATE kb_space SET
  name = 'K12 物理老师知识库',
  description = '面向中小学物理教师的范围说明、专业标准、模板与检查清单。主题：education、k12、physics、experiment。',
  updated_at = TIMESTAMPTZ '2026-11-03T00:00:00+08:00'
WHERE id = 533000000000000012 AND tenant_id = 100001 AND organization_id = 0;

UPDATE kb_space SET
  name = 'K12 化学老师知识库',
  description = '面向中小学化学教师的范围说明、专业标准、模板与检查清单。主题：education、k12、chemistry、experiment。',
  updated_at = TIMESTAMPTZ '2026-11-03T00:00:00+08:00'
WHERE id = 533000000000000013 AND tenant_id = 100001 AND organization_id = 0;

UPDATE kb_space SET
  name = 'K12 生物老师知识库',
  description = '面向中小学生物教师的范围说明、专业标准、模板与检查清单。主题：education、k12、biology、genetics、experiment。',
  updated_at = TIMESTAMPTZ '2026-11-03T00:00:00+08:00'
WHERE id = 533000000000000014 AND tenant_id = 100001 AND organization_id = 0;

UPDATE kb_space SET
  name = 'K12 历史老师知识库',
  description = '面向中小学历史教师的范围说明、专业标准、模板与检查清单。主题：education、k12、history、source-analysis。',
  updated_at = TIMESTAMPTZ '2026-11-03T00:00:00+08:00'
WHERE id = 533000000000000015 AND tenant_id = 100001 AND organization_id = 0;

UPDATE kb_space SET
  name = 'K12 地理老师知识库',
  description = '面向中小学地理教师的范围说明、专业标准、模板与检查清单。主题：education、k12、geography、map、region。',
  updated_at = TIMESTAMPTZ '2026-11-03T00:00:00+08:00'
WHERE id = 533000000000000016 AND tenant_id = 100001 AND organization_id = 0;

UPDATE kb_space SET
  name = 'K12 道德与法治老师知识库',
  description = '面向中小学道德与法治教师的范围说明、专业标准、模板与检查清单。主题：education、k12、politics、civics、law。',
  updated_at = TIMESTAMPTZ '2026-11-03T00:00:00+08:00'
WHERE id = 533000000000000017 AND tenant_id = 100001 AND organization_id = 0;

UPDATE kb_space SET
  name = '后端开发工程师知识库',
  description = '面向后端服务工程师的范围说明、专业标准、模板与检查清单。主题：engineering、programmer、backend、api、database。',
  updated_at = TIMESTAMPTZ '2026-11-03T00:00:00+08:00'
WHERE id = 533000000000000018 AND tenant_id = 100001 AND organization_id = 0;

UPDATE kb_space SET
  name = '软件架构师知识库',
  description = '面向软件架构师的范围说明、专业标准、模板与检查清单。主题：engineering、programmer、architecture、system-design。',
  updated_at = TIMESTAMPTZ '2026-11-03T00:00:00+08:00'
WHERE id = 533000000000000019 AND tenant_id = 100001 AND organization_id = 0;

UPDATE kb_space SET
  name = 'DevOps / SRE 工程师知识库',
  description = '面向DevOps 与 SRE 工程师的范围说明、专业标准、模板与检查清单。主题：engineering、programmer、devops、sre、ci-cd、kubernetes。',
  updated_at = TIMESTAMPTZ '2026-11-03T00:00:00+08:00'
WHERE id = 533000000000000020 AND tenant_id = 100001 AND organization_id = 0;

UPDATE kb_space SET
  name = '测试与质量工程师知识库',
  description = '面向测试与质量工程师的范围说明、专业标准、模板与检查清单。主题：engineering、programmer、qa、testing、quality、automation。',
  updated_at = TIMESTAMPTZ '2026-11-03T00:00:00+08:00'
WHERE id = 533000000000000021 AND tenant_id = 100001 AND organization_id = 0;

UPDATE kb_space SET
  name = '数据与 AI 工程师知识库',
  description = '面向数据与人工智能工程师的范围说明、专业标准、模板与检查清单。主题：engineering、programmer、data、ai、ml、evaluation。',
  updated_at = TIMESTAMPTZ '2026-11-03T00:00:00+08:00'
WHERE id = 533000000000000022 AND tenant_id = 100001 AND organization_id = 0;

UPDATE kb_space SET
  name = '大学数学导师知识库',
  description = '面向大学数学课程导师的范围说明、专业标准、模板与检查清单。主题：education、university、mathematics、proof。',
  updated_at = TIMESTAMPTZ '2026-11-03T00:00:00+08:00'
WHERE id = 533000000000000023 AND tenant_id = 100001 AND organization_id = 0;

UPDATE kb_space SET
  name = '大学物理导师知识库',
  description = '面向大学物理课程导师的范围说明、专业标准、模板与检查清单。主题：education、university、physics、mechanics、electromagnetism。',
  updated_at = TIMESTAMPTZ '2026-11-03T00:00:00+08:00'
WHERE id = 533000000000000024 AND tenant_id = 100001 AND organization_id = 0;

UPDATE kb_space SET
  name = '大学经济学导师知识库',
  description = '面向大学经济学课程导师的范围说明、专业标准、模板与检查清单。主题：education、university、economics、microeconomics、macroeconomics。',
  updated_at = TIMESTAMPTZ '2026-11-03T00:00:00+08:00'
WHERE id = 533000000000000025 AND tenant_id = 100001 AND organization_id = 0;

UPDATE kb_space SET
  name = '大学法学导师知识库',
  description = '面向大学法学课程导师的范围说明、专业标准、模板与检查清单。主题：education、university、law、case-method。',
  updated_at = TIMESTAMPTZ '2026-11-03T00:00:00+08:00'
WHERE id = 533000000000000026 AND tenant_id = 100001 AND organization_id = 0;

UPDATE kb_space SET
  name = '大学医学导师知识库',
  description = '面向大学医学课程导师的范围说明、专业标准、模板与检查清单。主题：education、university、medicine、preclinical、clinical-reasoning。',
  updated_at = TIMESTAMPTZ '2026-11-03T00:00:00+08:00'
WHERE id = 533000000000000027 AND tenant_id = 100001 AND organization_id = 0;

UPDATE kb_space SET
  name = '大学工程技术导师知识库',
  description = '面向大学工程技术课程导师的范围说明、专业标准、模板与检查清单。主题：education、university、engineering、mechanics、design。',
  updated_at = TIMESTAMPTZ '2026-11-03T00:00:00+08:00'
WHERE id = 533000000000000028 AND tenant_id = 100001 AND organization_id = 0;

UPDATE kb_space SET
  name = '儿科医学顾问知识库',
  description = '面向儿科医学顾问的范围说明、专业标准、模板与检查清单。主题：healthcare、doctor、pediatrics、child-health、triage。',
  updated_at = TIMESTAMPTZ '2026-11-03T00:00:00+08:00'
WHERE id = 533000000000000029 AND tenant_id = 100001 AND organization_id = 0;

UPDATE kb_space SET
  name = '心血管医学顾问知识库',
  description = '面向心血管医学顾问的范围说明、专业标准、模板与检查清单。主题：healthcare、doctor、cardiology、heart、chronic-care。',
  updated_at = TIMESTAMPTZ '2026-11-03T00:00:00+08:00'
WHERE id = 533000000000000030 AND tenant_id = 100001 AND organization_id = 0;

UPDATE kb_space SET
  name = '心理健康顾问知识库',
  description = '面向心理健康与心理教育顾问的范围说明、专业标准、模板与检查清单。主题：healthcare、doctor、mental-health、psychology、crisis。',
  updated_at = TIMESTAMPTZ '2026-11-03T00:00:00+08:00'
WHERE id = 533000000000000031 AND tenant_id = 100001 AND organization_id = 0;

UPDATE kb_space SET
  name = '劳动法律师知识库',
  description = '面向中国劳动法律师的范围说明、专业标准、模板与检查清单。主题：legal、lawyer、labor、employment、arbitration。',
  updated_at = TIMESTAMPTZ '2026-11-03T00:00:00+08:00'
WHERE id = 533000000000000032 AND tenant_id = 100001 AND organization_id = 0;

UPDATE kb_space SET
  name = '刑事辩护律师知识库',
  description = '面向中国刑事辩护法律顾问的范围说明、专业标准、模板与检查清单。主题：legal、lawyer、criminal、defense、procedure。',
  updated_at = TIMESTAMPTZ '2026-11-03T00:00:00+08:00'
WHERE id = 533000000000000033 AND tenant_id = 100001 AND organization_id = 0;

UPDATE kb_space SET
  name = '知识产权律师知识库',
  description = '面向中国知识产权法律顾问的范围说明、专业标准、模板与检查清单。主题：legal、lawyer、intellectual-property、trademark、patent、copyright。',
  updated_at = TIMESTAMPTZ '2026-11-03T00:00:00+08:00'
WHERE id = 533000000000000034 AND tenant_id = 100001 AND organization_id = 0;

UPDATE kb_space SET
  name = '婚姻家事律师知识库',
  description = '面向中国婚姻家事法律顾问的范围说明、专业标准、模板与检查清单。主题：legal、lawyer、family、marriage、custody、inheritance。',
  updated_at = TIMESTAMPTZ '2026-11-03T00:00:00+08:00'
WHERE id = 533000000000000035 AND tenant_id = 100001 AND organization_id = 0;

UPDATE kb_space SET
  name = '目的地向导知识库',
  description = '面向目的地文化与本地向导的范围说明、专业标准、模板与检查清单。主题：travel、guide、culture、local。',
  updated_at = TIMESTAMPTZ '2026-11-03T00:00:00+08:00'
WHERE id = 533000000000000036 AND tenant_id = 100001 AND organization_id = 0;

UPDATE kb_space SET
  name = '签证顾问知识库',
  description = '面向签证与入境事务顾问的范围说明、专业标准、模板与检查清单。主题：travel、visa、immigration、documents。',
  updated_at = TIMESTAMPTZ '2026-11-03T00:00:00+08:00'
WHERE id = 533000000000000037 AND tenant_id = 100001 AND organization_id = 0;

UPDATE kb_space SET
  name = '电商选品顾问知识库',
  description = '面向电商选品与市场研究顾问的范围说明、专业标准、模板与检查清单。主题：ecommerce、product-selection、sourcing、market-research。',
  updated_at = TIMESTAMPTZ '2026-11-03T00:00:00+08:00'
WHERE id = 533000000000000038 AND tenant_id = 100001 AND organization_id = 0;

UPDATE kb_space SET
  name = '电商客服专家知识库',
  description = '面向电商客服与售后专家的范围说明、专业标准、模板与检查清单。主题：ecommerce、customer-service、retention、service。',
  updated_at = TIMESTAMPTZ '2026-11-03T00:00:00+08:00'
WHERE id = 533000000000000039 AND tenant_id = 100001 AND organization_id = 0;

UPDATE kb_space SET
  name = '电商广告投放专家知识库',
  description = '面向电商广告与流量投放专家的范围说明、专业标准、模板与检查清单。主题：ecommerce、advertising、roi、traffic。',
  updated_at = TIMESTAMPTZ '2026-11-03T00:00:00+08:00'
WHERE id = 533000000000000040 AND tenant_id = 100001 AND organization_id = 0;

UPDATE kb_space SET
  name = '租房顾问知识库',
  description = '面向租房与租赁事务顾问的范围说明、专业标准、模板与检查清单。主题：realestate、rental、lease、tenant。',
  updated_at = TIMESTAMPTZ '2026-11-03T00:00:00+08:00'
WHERE id = 533000000000000041 AND tenant_id = 100001 AND organization_id = 0;

UPDATE kb_space SET
  name = '房产估值师知识库',
  description = '面向房产估值与投资分析顾问的范围说明、专业标准、模板与检查清单。主题：realestate、valuation、comparables、investment。',
  updated_at = TIMESTAMPTZ '2026-11-03T00:00:00+08:00'
WHERE id = 533000000000000042 AND tenant_id = 100001 AND organization_id = 0;

UPDATE kb_space SET
  name = '餐饮运营顾问知识库',
  description = '面向餐饮门店运营顾问的范围说明、专业标准、模板与检查清单。主题：restaurant、fnb、operations、menu-cost。',
  updated_at = TIMESTAMPTZ '2026-11-03T00:00:00+08:00'
WHERE id = 533000000000000043 AND tenant_id = 100001 AND organization_id = 0;

UPDATE kb_space SET
  name = '菜品研发与菜单工程师知识库',
  description = '面向菜品研发与菜单工程顾问的范围说明、专业标准、模板与检查清单。主题：restaurant、menu、recipe、costing。',
  updated_at = TIMESTAMPTZ '2026-11-03T00:00:00+08:00'
WHERE id = 533000000000000044 AND tenant_id = 100001 AND organization_id = 0;

UPDATE kb_space SET
  name = '食品安全与卫生顾问知识库',
  description = '面向餐饮食品安全与卫生顾问的范围说明、专业标准、模板与检查清单。主题：restaurant、food-safety、haccp、hygiene。',
  updated_at = TIMESTAMPTZ '2026-11-03T00:00:00+08:00'
WHERE id = 533000000000000045 AND tenant_id = 100001 AND organization_id = 0;

UPDATE kb_space SET
  name = '餐饮营销与外卖运营顾问知识库',
  description = '面向餐饮营销与外卖运营顾问的范围说明、专业标准、模板与检查清单。主题：restaurant、marketing、delivery、reviews。',
  updated_at = TIMESTAMPTZ '2026-11-03T00:00:00+08:00'
WHERE id = 533000000000000046 AND tenant_id = 100001 AND organization_id = 0;

UPDATE kb_space SET
  name = '企业管理咨询顾问知识库',
  description = '面向企业管理与运营咨询顾问的范围说明、专业标准、模板与检查清单。主题：enterprise、consulting、operating-model、process。',
  updated_at = TIMESTAMPTZ '2026-11-03T00:00:00+08:00'
WHERE id = 533000000000000047 AND tenant_id = 100001 AND organization_id = 0;

UPDATE kb_space SET
  name = '企业人力资源顾问知识库',
  description = '面向企业人力资源与组织发展顾问的范围说明、专业标准、模板与检查清单。主题：enterprise、hr、recruiting、performance。',
  updated_at = TIMESTAMPTZ '2026-11-03T00:00:00+08:00'
WHERE id = 533000000000000048 AND tenant_id = 100001 AND organization_id = 0;

UPDATE kb_space SET
  name = '企业财税顾问知识库',
  description = '面向企业财税与合规顾问的范围说明、专业标准、模板与检查清单。主题：enterprise、tax、finance、compliance。',
  updated_at = TIMESTAMPTZ '2026-11-03T00:00:00+08:00'
WHERE id = 533000000000000049 AND tenant_id = 100001 AND organization_id = 0;

UPDATE kb_space SET
  name = '企业服务销售顾问知识库',
  description = '面向B2B 企业服务销售顾问的范围说明、专业标准、模板与检查清单。主题：enterprise、b2b、sales、account-plan。',
  updated_at = TIMESTAMPTZ '2026-11-03T00:00:00+08:00'
WHERE id = 533000000000000050 AND tenant_id = 100001 AND organization_id = 0;

UPDATE kb_space SET
  name = '政务服务办事顾问知识库',
  description = '面向政务服务与办事指南顾问的范围说明、专业标准、模板与检查清单。主题：government、public-service、procedure、materials。',
  updated_at = TIMESTAMPTZ '2026-11-03T00:00:00+08:00'
WHERE id = 533000000000000051 AND tenant_id = 100001 AND organization_id = 0;

UPDATE kb_space SET
  name = '政策解读顾问知识库',
  description = '面向公共政策解读与合规顾问的范围说明、专业标准、模板与检查清单。主题：government、policy、interpretation、compliance。',
  updated_at = TIMESTAMPTZ '2026-11-03T00:00:00+08:00'
WHERE id = 533000000000000052 AND tenant_id = 100001 AND organization_id = 0;

UPDATE kb_space SET
  name = '政务公文写作顾问知识库',
  description = '面向政务公文写作与审核顾问的范围说明、专业标准、模板与检查清单。主题：government、official-document、writing、review。',
  updated_at = TIMESTAMPTZ '2026-11-03T00:00:00+08:00'
WHERE id = 533000000000000053 AND tenant_id = 100001 AND organization_id = 0;

UPDATE kb_space SET
  name = '数字政务解决方案顾问知识库',
  description = '面向数字政务与智慧城市解决方案顾问的范围说明、专业标准、模板与检查清单。主题：government、digital-government、solution、data-governance。',
  updated_at = TIMESTAMPTZ '2026-11-03T00:00:00+08:00'
WHERE id = 533000000000000054 AND tenant_id = 100001 AND organization_id = 0;

UPDATE kb_space SET
  name = '酒店运营顾问知识库',
  description = '面向酒店运营与宾客体验顾问的范围说明、专业标准、模板与检查清单。主题：hospitality、hotel、operations、guest-experience。',
  updated_at = TIMESTAMPTZ '2026-11-03T00:00:00+08:00'
WHERE id = 533000000000000055 AND tenant_id = 100001 AND organization_id = 0;

UPDATE kb_space SET
  name = '酒店收益管理顾问知识库',
  description = '面向酒店收益管理与定价顾问的范围说明、专业标准、模板与检查清单。主题：hospitality、hotel、revenue-management、pricing、forecast。',
  updated_at = TIMESTAMPTZ '2026-11-03T00:00:00+08:00'
WHERE id = 533000000000000056 AND tenant_id = 100001 AND organization_id = 0;

UPDATE kb_space SET
  name = '酒店前厅与客服顾问知识库',
  description = '面向酒店前厅与宾客服务顾问的范围说明、专业标准、模板与检查清单。主题：hospitality、hotel、guest-service、front-desk、complaint。',
  updated_at = TIMESTAMPTZ '2026-11-03T00:00:00+08:00'
WHERE id = 533000000000000057 AND tenant_id = 100001 AND organization_id = 0;

UPDATE kb_space SET
  name = '酒店营销与 OTA 顾问知识库',
  description = '面向酒店营销与 OTA 渠道顾问的范围说明、专业标准、模板与检查清单。主题：hospitality、hotel、marketing、ota、distribution。',
  updated_at = TIMESTAMPTZ '2026-11-03T00:00:00+08:00'
WHERE id = 533000000000000058 AND tenant_id = 100001 AND organization_id = 0;

UPDATE kb_space SET
  name = '小说创作家知识库',
  description = '面向长篇小说创作顾问的范围说明、专业标准、模板与检查清单。主题：writing、novel、fiction、story、serial。',
  updated_at = TIMESTAMPTZ '2026-11-03T00:00:00+08:00'
WHERE id = 533000000000000059 AND tenant_id = 100001 AND organization_id = 0;

UPDATE kb_space SET
  name = '图书作者知识库',
  description = '面向非虚构图书写作顾问的范围说明、专业标准、模板与检查清单。主题：writing、book、nonfiction、outline、publishing。',
  updated_at = TIMESTAMPTZ '2026-11-03T00:00:00+08:00'
WHERE id = 533000000000000060 AND tenant_id = 100001 AND organization_id = 0;

UPDATE kb_space SET
  name = '新闻记者知识库',
  description = '面向新闻采写与事实核查顾问的范围说明、专业标准、模板与检查清单。主题：writing、news、reporting、interview、fact-check。',
  updated_at = TIMESTAMPTZ '2026-11-03T00:00:00+08:00'
WHERE id = 533000000000000061 AND tenant_id = 100001 AND organization_id = 0;

UPDATE kb_space SET
  name = '文章作者知识库',
  description = '面向长篇内容与公众号写作顾问的范围说明、专业标准、模板与检查清单。主题：writing、article、blog、wechat、content。',
  updated_at = TIMESTAMPTZ '2026-11-03T00:00:00+08:00'
WHERE id = 533000000000000062 AND tenant_id = 100001 AND organization_id = 0;

UPDATE kb_space SET
  name = '文案策划知识库',
  description = '面向营销文案与转化写作顾问的范围说明、专业标准、模板与检查清单。主题：writing、copywriting、marketing、headline、conversion。',
  updated_at = TIMESTAMPTZ '2026-11-03T00:00:00+08:00'
WHERE id = 533000000000000063 AND tenant_id = 100001 AND organization_id = 0;

UPDATE kb_space SET
  name = '责任编辑知识库',
  description = '面向文稿编辑与审校顾问的范围说明、专业标准、模板与检查清单。主题：writing、editing、revision、proofreading、style。',
  updated_at = TIMESTAMPTZ '2026-11-03T00:00:00+08:00'
WHERE id = 533000000000000064 AND tenant_id = 100001 AND organization_id = 0;

COMMIT;
