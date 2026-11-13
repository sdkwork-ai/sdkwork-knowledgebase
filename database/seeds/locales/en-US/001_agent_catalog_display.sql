-- SDKWork official agent catalog: en-US knowledge-space display.

-- Catalog revision: 1.3.0.

BEGIN;

SELECT set_config('app.current_tenant_id', '100001', true);

SELECT set_config('app.current_organization_id', '0', true);

UPDATE kb_space SET
  name = 'Full-Stack Engineer Knowledge Base',
  description = 'Curated scope, standards, templates, and checklists for full-stack software engineer. Topics: engineering, programmer, fullstack, web, api.',
  updated_at = TIMESTAMPTZ '2026-11-03T00:00:00+08:00'
WHERE id = 533000000000000001 AND tenant_id = 100001 AND organization_id = 0;

UPDATE kb_space SET
  name = 'Frontend Engineer Knowledge Base',
  description = 'Curated scope, standards, templates, and checklists for frontend engineer. Topics: engineering, programmer, frontend, react, accessibility.',
  updated_at = TIMESTAMPTZ '2026-11-03T00:00:00+08:00'
WHERE id = 533000000000000002 AND tenant_id = 100001 AND organization_id = 0;

UPDATE kb_space SET
  name = 'K12 Math Teacher Knowledge Base',
  description = 'Curated scope, standards, templates, and checklists for K-12 mathematics teacher. Topics: education, k12, math, curriculum.',
  updated_at = TIMESTAMPTZ '2026-11-03T00:00:00+08:00'
WHERE id = 533000000000000003 AND tenant_id = 100001 AND organization_id = 0;

UPDATE kb_space SET
  name = 'University Computer Science Mentor Knowledge Base',
  description = 'Curated scope, standards, templates, and checklists for computer science course mentor. Topics: education, university, computer-science, algorithms.',
  updated_at = TIMESTAMPTZ '2026-11-03T00:00:00+08:00'
WHERE id = 533000000000000004 AND tenant_id = 100001 AND organization_id = 0;

UPDATE kb_space SET
  name = 'General Practice Advisor Knowledge Base',
  description = 'Curated scope, standards, templates, and checklists for general practice physician advisor. Topics: healthcare, doctor, general-practice, triage, health.',
  updated_at = TIMESTAMPTZ '2026-11-03T00:00:00+08:00'
WHERE id = 533000000000000005 AND tenant_id = 100001 AND organization_id = 0;

UPDATE kb_space SET
  name = 'Civil and Commercial Lawyer Knowledge Base',
  description = 'Curated scope, standards, templates, and checklists for PRC civil and commercial legal advisor. Topics: legal, lawyer, civil, contract, dispute.',
  updated_at = TIMESTAMPTZ '2026-11-03T00:00:00+08:00'
WHERE id = 533000000000000006 AND tenant_id = 100001 AND organization_id = 0;

UPDATE kb_space SET
  name = 'Travel Planner Knowledge Base',
  description = 'Curated scope, standards, templates, and checklists for travel planner. Topics: travel, itinerary, budget, planning.',
  updated_at = TIMESTAMPTZ '2026-11-03T00:00:00+08:00'
WHERE id = 533000000000000007 AND tenant_id = 100001 AND organization_id = 0;

UPDATE kb_space SET
  name = 'E-commerce Operations Advisor Knowledge Base',
  description = 'Curated scope, standards, templates, and checklists for e-commerce operations advisor. Topics: ecommerce, operations, growth, conversion.',
  updated_at = TIMESTAMPTZ '2026-11-03T00:00:00+08:00'
WHERE id = 533000000000000008 AND tenant_id = 100001 AND organization_id = 0;

UPDATE kb_space SET
  name = 'Home Buyer Advisor Knowledge Base',
  description = 'Curated scope, standards, templates, and checklists for home buyer advisor. Topics: realestate, buyer, due-diligence, negotiation.',
  updated_at = TIMESTAMPTZ '2026-11-03T00:00:00+08:00'
WHERE id = 533000000000000009 AND tenant_id = 100001 AND organization_id = 0;

UPDATE kb_space SET
  name = 'K12 Chinese Language Teacher Knowledge Base',
  description = 'Curated scope, standards, templates, and checklists for K-12 Chinese language teacher. Topics: education, k12, chinese, reading, writing.',
  updated_at = TIMESTAMPTZ '2026-11-03T00:00:00+08:00'
WHERE id = 533000000000000010 AND tenant_id = 100001 AND organization_id = 0;

UPDATE kb_space SET
  name = 'K12 English Teacher Knowledge Base',
  description = 'Curated scope, standards, templates, and checklists for K-12 English teacher. Topics: education, k12, english, listening, writing.',
  updated_at = TIMESTAMPTZ '2026-11-03T00:00:00+08:00'
WHERE id = 533000000000000011 AND tenant_id = 100001 AND organization_id = 0;

UPDATE kb_space SET
  name = 'K12 Physics Teacher Knowledge Base',
  description = 'Curated scope, standards, templates, and checklists for K-12 physics teacher. Topics: education, k12, physics, experiment.',
  updated_at = TIMESTAMPTZ '2026-11-03T00:00:00+08:00'
WHERE id = 533000000000000012 AND tenant_id = 100001 AND organization_id = 0;

UPDATE kb_space SET
  name = 'K12 Chemistry Teacher Knowledge Base',
  description = 'Curated scope, standards, templates, and checklists for K-12 chemistry teacher. Topics: education, k12, chemistry, experiment.',
  updated_at = TIMESTAMPTZ '2026-11-03T00:00:00+08:00'
WHERE id = 533000000000000013 AND tenant_id = 100001 AND organization_id = 0;

UPDATE kb_space SET
  name = 'K12 Biology Teacher Knowledge Base',
  description = 'Curated scope, standards, templates, and checklists for K-12 biology teacher. Topics: education, k12, biology, genetics, experiment.',
  updated_at = TIMESTAMPTZ '2026-11-03T00:00:00+08:00'
WHERE id = 533000000000000014 AND tenant_id = 100001 AND organization_id = 0;

UPDATE kb_space SET
  name = 'K12 History Teacher Knowledge Base',
  description = 'Curated scope, standards, templates, and checklists for K-12 history teacher. Topics: education, k12, history, source-analysis.',
  updated_at = TIMESTAMPTZ '2026-11-03T00:00:00+08:00'
WHERE id = 533000000000000015 AND tenant_id = 100001 AND organization_id = 0;

UPDATE kb_space SET
  name = 'K12 Geography Teacher Knowledge Base',
  description = 'Curated scope, standards, templates, and checklists for K-12 geography teacher. Topics: education, k12, geography, map, region.',
  updated_at = TIMESTAMPTZ '2026-11-03T00:00:00+08:00'
WHERE id = 533000000000000016 AND tenant_id = 100001 AND organization_id = 0;

UPDATE kb_space SET
  name = 'K12 Ethics and Law Teacher Knowledge Base',
  description = 'Curated scope, standards, templates, and checklists for K-12 ethics and law teacher. Topics: education, k12, politics, civics, law.',
  updated_at = TIMESTAMPTZ '2026-11-03T00:00:00+08:00'
WHERE id = 533000000000000017 AND tenant_id = 100001 AND organization_id = 0;

UPDATE kb_space SET
  name = 'Backend Engineer Knowledge Base',
  description = 'Curated scope, standards, templates, and checklists for backend service engineer. Topics: engineering, programmer, backend, api, database.',
  updated_at = TIMESTAMPTZ '2026-11-03T00:00:00+08:00'
WHERE id = 533000000000000018 AND tenant_id = 100001 AND organization_id = 0;

UPDATE kb_space SET
  name = 'Software Architect Knowledge Base',
  description = 'Curated scope, standards, templates, and checklists for software architect. Topics: engineering, programmer, architecture, system-design.',
  updated_at = TIMESTAMPTZ '2026-11-03T00:00:00+08:00'
WHERE id = 533000000000000019 AND tenant_id = 100001 AND organization_id = 0;

UPDATE kb_space SET
  name = 'DevOps and SRE Engineer Knowledge Base',
  description = 'Curated scope, standards, templates, and checklists for DevOps and site reliability engineer. Topics: engineering, programmer, devops, sre, ci-cd, kubernetes.',
  updated_at = TIMESTAMPTZ '2026-11-03T00:00:00+08:00'
WHERE id = 533000000000000020 AND tenant_id = 100001 AND organization_id = 0;

UPDATE kb_space SET
  name = 'QA and Quality Engineer Knowledge Base',
  description = 'Curated scope, standards, templates, and checklists for QA and quality engineer. Topics: engineering, programmer, qa, testing, quality, automation.',
  updated_at = TIMESTAMPTZ '2026-11-03T00:00:00+08:00'
WHERE id = 533000000000000021 AND tenant_id = 100001 AND organization_id = 0;

UPDATE kb_space SET
  name = 'Data and AI Engineer Knowledge Base',
  description = 'Curated scope, standards, templates, and checklists for data and AI engineer. Topics: engineering, programmer, data, ai, ml, evaluation.',
  updated_at = TIMESTAMPTZ '2026-11-03T00:00:00+08:00'
WHERE id = 533000000000000022 AND tenant_id = 100001 AND organization_id = 0;

UPDATE kb_space SET
  name = 'University Mathematics Mentor Knowledge Base',
  description = 'Curated scope, standards, templates, and checklists for university mathematics course mentor. Topics: education, university, mathematics, proof.',
  updated_at = TIMESTAMPTZ '2026-11-03T00:00:00+08:00'
WHERE id = 533000000000000023 AND tenant_id = 100001 AND organization_id = 0;

UPDATE kb_space SET
  name = 'University Physics Mentor Knowledge Base',
  description = 'Curated scope, standards, templates, and checklists for university physics course mentor. Topics: education, university, physics, mechanics, electromagnetism.',
  updated_at = TIMESTAMPTZ '2026-11-03T00:00:00+08:00'
WHERE id = 533000000000000024 AND tenant_id = 100001 AND organization_id = 0;

UPDATE kb_space SET
  name = 'University Economics Mentor Knowledge Base',
  description = 'Curated scope, standards, templates, and checklists for university economics course mentor. Topics: education, university, economics, microeconomics, macroeconomics.',
  updated_at = TIMESTAMPTZ '2026-11-03T00:00:00+08:00'
WHERE id = 533000000000000025 AND tenant_id = 100001 AND organization_id = 0;

UPDATE kb_space SET
  name = 'University Law Mentor Knowledge Base',
  description = 'Curated scope, standards, templates, and checklists for university law course mentor. Topics: education, university, law, case-method.',
  updated_at = TIMESTAMPTZ '2026-11-03T00:00:00+08:00'
WHERE id = 533000000000000026 AND tenant_id = 100001 AND organization_id = 0;

UPDATE kb_space SET
  name = 'University Medicine Mentor Knowledge Base',
  description = 'Curated scope, standards, templates, and checklists for university medicine course mentor. Topics: education, university, medicine, preclinical, clinical-reasoning.',
  updated_at = TIMESTAMPTZ '2026-11-03T00:00:00+08:00'
WHERE id = 533000000000000027 AND tenant_id = 100001 AND organization_id = 0;

UPDATE kb_space SET
  name = 'University Engineering Mentor Knowledge Base',
  description = 'Curated scope, standards, templates, and checklists for university engineering course mentor. Topics: education, university, engineering, mechanics, design.',
  updated_at = TIMESTAMPTZ '2026-11-03T00:00:00+08:00'
WHERE id = 533000000000000028 AND tenant_id = 100001 AND organization_id = 0;

UPDATE kb_space SET
  name = 'Pediatric Health Advisor Knowledge Base',
  description = 'Curated scope, standards, templates, and checklists for pediatric health advisor. Topics: healthcare, doctor, pediatrics, child-health, triage.',
  updated_at = TIMESTAMPTZ '2026-11-03T00:00:00+08:00'
WHERE id = 533000000000000029 AND tenant_id = 100001 AND organization_id = 0;

UPDATE kb_space SET
  name = 'Cardiology Health Advisor Knowledge Base',
  description = 'Curated scope, standards, templates, and checklists for cardiology health advisor. Topics: healthcare, doctor, cardiology, heart, chronic-care.',
  updated_at = TIMESTAMPTZ '2026-11-03T00:00:00+08:00'
WHERE id = 533000000000000030 AND tenant_id = 100001 AND organization_id = 0;

UPDATE kb_space SET
  name = 'Mental Health Advisor Knowledge Base',
  description = 'Curated scope, standards, templates, and checklists for mental health and psychoeducation advisor. Topics: healthcare, doctor, mental-health, psychology, crisis.',
  updated_at = TIMESTAMPTZ '2026-11-03T00:00:00+08:00'
WHERE id = 533000000000000031 AND tenant_id = 100001 AND organization_id = 0;

UPDATE kb_space SET
  name = 'Labor and Employment Lawyer Knowledge Base',
  description = 'Curated scope, standards, templates, and checklists for PRC labor and employment legal advisor. Topics: legal, lawyer, labor, employment, arbitration.',
  updated_at = TIMESTAMPTZ '2026-11-03T00:00:00+08:00'
WHERE id = 533000000000000032 AND tenant_id = 100001 AND organization_id = 0;

UPDATE kb_space SET
  name = 'Criminal Defense Lawyer Knowledge Base',
  description = 'Curated scope, standards, templates, and checklists for PRC criminal defense legal advisor. Topics: legal, lawyer, criminal, defense, procedure.',
  updated_at = TIMESTAMPTZ '2026-11-03T00:00:00+08:00'
WHERE id = 533000000000000033 AND tenant_id = 100001 AND organization_id = 0;

UPDATE kb_space SET
  name = 'Intellectual Property Lawyer Knowledge Base',
  description = 'Curated scope, standards, templates, and checklists for PRC intellectual property legal advisor. Topics: legal, lawyer, intellectual-property, trademark, patent, copyright.',
  updated_at = TIMESTAMPTZ '2026-11-03T00:00:00+08:00'
WHERE id = 533000000000000034 AND tenant_id = 100001 AND organization_id = 0;

UPDATE kb_space SET
  name = 'Family and Matrimonial Lawyer Knowledge Base',
  description = 'Curated scope, standards, templates, and checklists for PRC family and matrimonial legal advisor. Topics: legal, lawyer, family, marriage, custody, inheritance.',
  updated_at = TIMESTAMPTZ '2026-11-03T00:00:00+08:00'
WHERE id = 533000000000000035 AND tenant_id = 100001 AND organization_id = 0;

UPDATE kb_space SET
  name = 'Destination Guide Knowledge Base',
  description = 'Curated scope, standards, templates, and checklists for destination culture and local guide. Topics: travel, guide, culture, local.',
  updated_at = TIMESTAMPTZ '2026-11-03T00:00:00+08:00'
WHERE id = 533000000000000036 AND tenant_id = 100001 AND organization_id = 0;

UPDATE kb_space SET
  name = 'Visa Advisor Knowledge Base',
  description = 'Curated scope, standards, templates, and checklists for visa and entry advisor. Topics: travel, visa, immigration, documents.',
  updated_at = TIMESTAMPTZ '2026-11-03T00:00:00+08:00'
WHERE id = 533000000000000037 AND tenant_id = 100001 AND organization_id = 0;

UPDATE kb_space SET
  name = 'E-commerce Product Scout Knowledge Base',
  description = 'Curated scope, standards, templates, and checklists for e-commerce product selection and market research advisor. Topics: ecommerce, product-selection, sourcing, market-research.',
  updated_at = TIMESTAMPTZ '2026-11-03T00:00:00+08:00'
WHERE id = 533000000000000038 AND tenant_id = 100001 AND organization_id = 0;

UPDATE kb_space SET
  name = 'E-commerce Customer Service Expert Knowledge Base',
  description = 'Curated scope, standards, templates, and checklists for e-commerce customer service and after-sales expert. Topics: ecommerce, customer-service, retention, service.',
  updated_at = TIMESTAMPTZ '2026-11-03T00:00:00+08:00'
WHERE id = 533000000000000039 AND tenant_id = 100001 AND organization_id = 0;

UPDATE kb_space SET
  name = 'E-commerce Advertising Specialist Knowledge Base',
  description = 'Curated scope, standards, templates, and checklists for e-commerce advertising and traffic specialist. Topics: ecommerce, advertising, roi, traffic.',
  updated_at = TIMESTAMPTZ '2026-11-03T00:00:00+08:00'
WHERE id = 533000000000000040 AND tenant_id = 100001 AND organization_id = 0;

UPDATE kb_space SET
  name = 'Rental Advisor Knowledge Base',
  description = 'Curated scope, standards, templates, and checklists for rental and lease advisor. Topics: realestate, rental, lease, tenant.',
  updated_at = TIMESTAMPTZ '2026-11-03T00:00:00+08:00'
WHERE id = 533000000000000041 AND tenant_id = 100001 AND organization_id = 0;

UPDATE kb_space SET
  name = 'Property Valuation Analyst Knowledge Base',
  description = 'Curated scope, standards, templates, and checklists for property valuation and investment analysis advisor. Topics: realestate, valuation, comparables, investment.',
  updated_at = TIMESTAMPTZ '2026-11-03T00:00:00+08:00'
WHERE id = 533000000000000042 AND tenant_id = 100001 AND organization_id = 0;

UPDATE kb_space SET
  name = 'Restaurant Operations Advisor Knowledge Base',
  description = 'Curated scope, standards, templates, and checklists for restaurant operations advisor. Topics: restaurant, fnb, operations, menu-cost.',
  updated_at = TIMESTAMPTZ '2026-11-03T00:00:00+08:00'
WHERE id = 533000000000000043 AND tenant_id = 100001 AND organization_id = 0;

UPDATE kb_space SET
  name = 'Menu Development and Menu Engineer Knowledge Base',
  description = 'Curated scope, standards, templates, and checklists for menu development and menu engineering advisor. Topics: restaurant, menu, recipe, costing.',
  updated_at = TIMESTAMPTZ '2026-11-03T00:00:00+08:00'
WHERE id = 533000000000000044 AND tenant_id = 100001 AND organization_id = 0;

UPDATE kb_space SET
  name = 'Food Safety and Hygiene Advisor Knowledge Base',
  description = 'Curated scope, standards, templates, and checklists for restaurant food safety and hygiene advisor. Topics: restaurant, food-safety, haccp, hygiene.',
  updated_at = TIMESTAMPTZ '2026-11-03T00:00:00+08:00'
WHERE id = 533000000000000045 AND tenant_id = 100001 AND organization_id = 0;

UPDATE kb_space SET
  name = 'Restaurant Marketing and Delivery Advisor Knowledge Base',
  description = 'Curated scope, standards, templates, and checklists for restaurant marketing and delivery operations advisor. Topics: restaurant, marketing, delivery, reviews.',
  updated_at = TIMESTAMPTZ '2026-11-03T00:00:00+08:00'
WHERE id = 533000000000000046 AND tenant_id = 100001 AND organization_id = 0;

UPDATE kb_space SET
  name = 'Enterprise Management Consultant Knowledge Base',
  description = 'Curated scope, standards, templates, and checklists for enterprise management and operations consultant. Topics: enterprise, consulting, operating-model, process.',
  updated_at = TIMESTAMPTZ '2026-11-03T00:00:00+08:00'
WHERE id = 533000000000000047 AND tenant_id = 100001 AND organization_id = 0;

UPDATE kb_space SET
  name = 'Enterprise HR Advisor Knowledge Base',
  description = 'Curated scope, standards, templates, and checklists for enterprise HR and organization development advisor. Topics: enterprise, hr, recruiting, performance.',
  updated_at = TIMESTAMPTZ '2026-11-03T00:00:00+08:00'
WHERE id = 533000000000000048 AND tenant_id = 100001 AND organization_id = 0;

UPDATE kb_space SET
  name = 'Enterprise Tax and Finance Advisor Knowledge Base',
  description = 'Curated scope, standards, templates, and checklists for enterprise tax and finance compliance advisor. Topics: enterprise, tax, finance, compliance.',
  updated_at = TIMESTAMPTZ '2026-11-03T00:00:00+08:00'
WHERE id = 533000000000000049 AND tenant_id = 100001 AND organization_id = 0;

UPDATE kb_space SET
  name = 'B2B Enterprise Sales Advisor Knowledge Base',
  description = 'Curated scope, standards, templates, and checklists for B2B enterprise services sales advisor. Topics: enterprise, b2b, sales, account-plan.',
  updated_at = TIMESTAMPTZ '2026-11-03T00:00:00+08:00'
WHERE id = 533000000000000050 AND tenant_id = 100001 AND organization_id = 0;

UPDATE kb_space SET
  name = 'Government Service Guide Advisor Knowledge Base',
  description = 'Curated scope, standards, templates, and checklists for government service and procedure guide advisor. Topics: government, public-service, procedure, materials.',
  updated_at = TIMESTAMPTZ '2026-11-03T00:00:00+08:00'
WHERE id = 533000000000000051 AND tenant_id = 100001 AND organization_id = 0;

UPDATE kb_space SET
  name = 'Public Policy Interpretation Advisor Knowledge Base',
  description = 'Curated scope, standards, templates, and checklists for public policy interpretation and compliance advisor. Topics: government, policy, interpretation, compliance.',
  updated_at = TIMESTAMPTZ '2026-11-03T00:00:00+08:00'
WHERE id = 533000000000000052 AND tenant_id = 100001 AND organization_id = 0;

UPDATE kb_space SET
  name = 'Government Official Document Advisor Knowledge Base',
  description = 'Curated scope, standards, templates, and checklists for government official document writing and review advisor. Topics: government, official-document, writing, review.',
  updated_at = TIMESTAMPTZ '2026-11-03T00:00:00+08:00'
WHERE id = 533000000000000053 AND tenant_id = 100001 AND organization_id = 0;

UPDATE kb_space SET
  name = 'Digital Government Solution Advisor Knowledge Base',
  description = 'Curated scope, standards, templates, and checklists for digital government and smart city solution advisor. Topics: government, digital-government, solution, data-governance.',
  updated_at = TIMESTAMPTZ '2026-11-03T00:00:00+08:00'
WHERE id = 533000000000000054 AND tenant_id = 100001 AND organization_id = 0;

UPDATE kb_space SET
  name = 'Hotel Operations Advisor Knowledge Base',
  description = 'Curated scope, standards, templates, and checklists for hotel operations and guest experience advisor. Topics: hospitality, hotel, operations, guest-experience.',
  updated_at = TIMESTAMPTZ '2026-11-03T00:00:00+08:00'
WHERE id = 533000000000000055 AND tenant_id = 100001 AND organization_id = 0;

UPDATE kb_space SET
  name = 'Hotel Revenue Management Advisor Knowledge Base',
  description = 'Curated scope, standards, templates, and checklists for hotel revenue management and pricing advisor. Topics: hospitality, hotel, revenue-management, pricing, forecast.',
  updated_at = TIMESTAMPTZ '2026-11-03T00:00:00+08:00'
WHERE id = 533000000000000056 AND tenant_id = 100001 AND organization_id = 0;

UPDATE kb_space SET
  name = 'Hotel Front Office and Guest Service Advisor Knowledge Base',
  description = 'Curated scope, standards, templates, and checklists for hotel front office and guest service advisor. Topics: hospitality, hotel, guest-service, front-desk, complaint.',
  updated_at = TIMESTAMPTZ '2026-11-03T00:00:00+08:00'
WHERE id = 533000000000000057 AND tenant_id = 100001 AND organization_id = 0;

UPDATE kb_space SET
  name = 'Hotel Marketing and OTA Advisor Knowledge Base',
  description = 'Curated scope, standards, templates, and checklists for hotel marketing and OTA distribution advisor. Topics: hospitality, hotel, marketing, ota, distribution.',
  updated_at = TIMESTAMPTZ '2026-11-03T00:00:00+08:00'
WHERE id = 533000000000000058 AND tenant_id = 100001 AND organization_id = 0;

UPDATE kb_space SET
  name = 'Novel Writer Knowledge Base',
  description = 'Curated scope, standards, templates, and checklists for long-form fiction writing advisor. Topics: writing, novel, fiction, story, serial.',
  updated_at = TIMESTAMPTZ '2026-11-03T00:00:00+08:00'
WHERE id = 533000000000000059 AND tenant_id = 100001 AND organization_id = 0;

UPDATE kb_space SET
  name = 'Book Author Knowledge Base',
  description = 'Curated scope, standards, templates, and checklists for nonfiction book writing advisor. Topics: writing, book, nonfiction, outline, publishing.',
  updated_at = TIMESTAMPTZ '2026-11-03T00:00:00+08:00'
WHERE id = 533000000000000060 AND tenant_id = 100001 AND organization_id = 0;

UPDATE kb_space SET
  name = 'News Reporter Knowledge Base',
  description = 'Curated scope, standards, templates, and checklists for news reporting and fact-checking advisor. Topics: writing, news, reporting, interview, fact-check.',
  updated_at = TIMESTAMPTZ '2026-11-03T00:00:00+08:00'
WHERE id = 533000000000000061 AND tenant_id = 100001 AND organization_id = 0;

UPDATE kb_space SET
  name = 'Article Writer Knowledge Base',
  description = 'Curated scope, standards, templates, and checklists for long-form content and WeChat article advisor. Topics: writing, article, blog, wechat, content.',
  updated_at = TIMESTAMPTZ '2026-11-03T00:00:00+08:00'
WHERE id = 533000000000000062 AND tenant_id = 100001 AND organization_id = 0;

UPDATE kb_space SET
  name = 'Copywriter Knowledge Base',
  description = 'Curated scope, standards, templates, and checklists for marketing copy and conversion writing advisor. Topics: writing, copywriting, marketing, headline, conversion.',
  updated_at = TIMESTAMPTZ '2026-11-03T00:00:00+08:00'
WHERE id = 533000000000000063 AND tenant_id = 100001 AND organization_id = 0;

UPDATE kb_space SET
  name = 'Editor Knowledge Base',
  description = 'Curated scope, standards, templates, and checklists for manuscript editing and proofreading advisor. Topics: writing, editing, revision, proofreading, style.',
  updated_at = TIMESTAMPTZ '2026-11-03T00:00:00+08:00'
WHERE id = 533000000000000064 AND tenant_id = 100001 AND organization_id = 0;

COMMIT;
