const TEMPLATE_FALLBACK_HINTS = [
  '由基础模板生成',
  '使用基础模板生成',
  '由基礎模板生成',
  '使用基礎模板生成',
  'generated from the base template',
];

function normalizeMode(mode) {
  return (mode || '').toString().trim().toLowerCase();
}

function normalizeModelName(modelName) {
  const trimmed = (modelName || '').toString().trim();
  return trimmed || null;
}

function containsTemplateFallbackHint(content) {
  const normalizedContent = (content || '').toString();
  const contentLower = normalizedContent.toLowerCase();

  return TEMPLATE_FALLBACK_HINTS.some((hint) => {
    const normalizedHint = hint.toLowerCase();
    return normalizedHint === 'generated from the base template'
      ? contentLower.includes(normalizedHint)
      : normalizedContent.includes(hint);
  });
}

export function resolveReportMeta(reportData, currentConfig) {
  const configMode = normalizeMode(currentConfig?.ai_mode);
  const configModelName =
    configMode === 'summary'
      ? normalizeModelName(currentConfig?.text_model?.model)
      : null;
  const fallbackReason = normalizeModelName(reportData?.fallback_reason);

  let reportMode = normalizeMode(reportData?.ai_mode || configMode);
  let reportModelName = normalizeModelName(reportData?.model_name);

  if (containsTemplateFallbackHint(reportData?.content)) {
    reportMode = 'local';
    reportModelName = null;
  }

  if (!reportData) {
    reportMode = configMode;
    reportModelName = configMode === 'summary' ? configModelName : null;
  }

  const showConfigMeta = Boolean(
    currentConfig &&
      (reportMode !== configMode ||
        reportModelName !== (configMode === 'summary' ? configModelName : null))
  );
  const showUsageMismatchNotice = configMode === 'summary' && reportMode === 'local';

  return {
    reportMode,
    reportModelName,
    configMode,
    configModelName,
    showConfigMeta,
    showUsageMismatchNotice,
    fallbackReason,
  };
}
