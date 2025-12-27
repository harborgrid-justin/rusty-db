module.exports = {
  root: true,
  parser: '@typescript-eslint/parser',
  parserOptions: {
    project: './tsconfig.json',
    ecmaVersion: 2022,
    sourceType: 'module',
  },
  plugins: ['@typescript-eslint'],
  extends: [
    'eslint:recommended',
    'plugin:@typescript-eslint/recommended',
    'prettier',
  ],
  env: {
    node: true,
    es2022: true,
  },
  rules: {
    // TypeScript-specific rules
    '@typescript-eslint/no-unused-vars': [
      'error',
      {
        argsIgnorePattern: '^_',
        varsIgnorePattern: '^_',
        caughtErrorsIgnorePattern: '^_',
      },
    ],
    '@typescript-eslint/no-explicit-any': 'error',
    '@typescript-eslint/explicit-function-return-type': [
      'warn',
      {
        allowExpressions: true,
        allowTypedFunctionExpressions: true,
        allowHigherOrderFunctions: true,
      },
    ],
    '@typescript-eslint/explicit-module-boundary-types': 'warn',

    // General JavaScript/TypeScript rules
    'no-console': 'warn',
    'prefer-const': 'error',

    // Additional enterprise-grade rules
    'no-var': 'error',
    'no-debugger': 'error',
    'no-duplicate-imports': 'error',
    'no-unused-expressions': 'error',
    'no-unreachable': 'error',
    'eqeqeq': ['error', 'always'],
    'curly': ['error', 'all'],
    'brace-style': ['error', '1tbs'],
    'no-throw-literal': 'error',
    'no-return-await': 'error',
  },
  ignorePatterns: [
    'dist',
    'node_modules',
    'coverage',
    'examples',
    '*.js',
    '*.cjs',
    '*.mjs',
  ],
};
