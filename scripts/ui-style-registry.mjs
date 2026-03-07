export const approvedUiHexColorRegistry = {
  themeCore: ['#3165D4'],
  appCanvas: ['#FBFBFB', '#111111'],
  sidebarSurface: ['#EDEDED', '#28282B'],
  sidebarInteraction: ['#E0E0E0', '#D8D8D8', '#E5E5E5', '#36373B', '#323338', '#303136'],
  guardActions: ['#D4313E', '#4AA658'],
  communityBrand: ['#5865F2', '#4752C4', '#3F49B7', '#7B86F8', '#A0A8FF'],
  walletPalette: [
    '#2563EB',
    '#4F46E5',
    '#7C3AED',
    '#A21CAF',
    '#DB2777',
    '#DC2626',
    '#EA580C',
    '#CA8A04',
    '#65A30D',
    '#16A34A',
    '#0F766E',
    '#0891B2',
    '#475569',
    '#78716C',
  ],
  identityAvatarGradients: ['#1D4ED8', '#1E3A8A', '#0EA5E9', '#0E7490', '#0284C7'],
};

export const approvedUiColorFunctionRegistry = {
  coinIconShadowStops: [
    'rgba(0, 0, 0, 1)',
    'rgba(0, 0, 0, 0.62)',
    'rgba(0, 0, 0, 0.1)',
    'rgba(0, 0, 0, 0)',
  ],
  overviewShadow: ['rgba(0,0,0,0.72)'],
};

export const approvedUiHexSources = {
  'src/app.css': [
    'themeCore',
    'appCanvas',
    'sidebarSurface',
    'sidebarInteraction',
    'guardActions',
    'communityBrand',
  ],
  'src/lib/constants/walletColors.ts': ['walletPalette'],
  'src/lib/styles/identityAvatarGradients.ts': ['walletPalette', 'identityAvatarGradients'],
};

export const approvedUiColorFunctionSources = {
  'src/app.css': ['coinIconShadowStops', 'overviewShadow'],
};

export const colorFunctionTokenFiles = new Set(['src/app.css']);
export const scanRoots = ['src'];
export const scanExtensions = new Set(['.css', '.js', '.mjs', '.svelte', '.ts']);
export const nativeControlUiRoot = 'src/lib/components/ui/';
export const governedNativeControls = ['input', 'textarea', 'select'];
export const governedFormPrimitives = ['label'];

export function normalizeHexColor(value) {
  return value.toUpperCase();
}

export function normalizeColorFunction(value) {
  return value.replace(/\s+/g, '').toLowerCase();
}

export const approvedUiHexColors = new Set(
  Object.values(approvedUiHexColorRegistry).flat().map(normalizeHexColor)
);

export const approvedUiColorFunctions = new Set(
  Object.values(approvedUiColorFunctionRegistry).flat().map(normalizeColorFunction)
);

export const approvedUiHexColorsBySource = new Map(
  Object.entries(approvedUiHexSources).map(([filePath, groups]) => [
    filePath,
    new Set(groups.flatMap((group) => approvedUiHexColorRegistry[group]).map(normalizeHexColor)),
  ])
);

export const approvedUiColorFunctionsBySource = new Map(
  Object.entries(approvedUiColorFunctionSources).map(([filePath, groups]) => [
    filePath,
    new Set(
      groups.flatMap((group) => approvedUiColorFunctionRegistry[group]).map(normalizeColorFunction)
    ),
  ])
);
