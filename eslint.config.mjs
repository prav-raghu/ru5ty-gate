import eslint from "@eslint/js";
import sonarjs from "eslint-plugin-sonarjs";
import unicorn from "eslint-plugin-unicorn";
import globals from "globals";
import tseslint from "typescript-eslint";

export default tseslint.config(
    {
        ignores: [
            "**/node_modules/**",
            "**/dist/**",
            "**/build/**",
            "**/coverage/**",
            "**/.turbo/**",
            "**/.next/**",
            "**/public/**",
            "**/*.config.js",
            "**/*.config.mjs",
            "**/jest.preset.js",
            "**/*.d.ts",
            "**/tailwind.config.ts",
            "**/vite.config.ts",
            "**/next.config.ts",
            "**/cypress.config.ts",
            "apps/cms/.strapi/**",
            "apps/cms/config/**",
            "apps/cms/src/admin/**",
            "devops/scripts/**",
        ],
    },

    eslint.configs.recommended,

    ...tseslint.configs.recommended,
    ...tseslint.configs.stylistic,

    {
        plugins: {
            unicorn,
            sonarjs,
        },
    },

    {
        files: ["**/*.ts", "**/*.tsx"],
        languageOptions: {
            ecmaVersion: "latest",
            sourceType: "module",
            globals: {
                ...globals.node,
                ...globals.es2024,
            },
            parserOptions: {
                tsconfigRootDir: import.meta.dirname,
            },
        },
        rules: {
            "@typescript-eslint/no-explicit-any": "error",
            "@typescript-eslint/explicit-function-return-type": [
                "warn",
                {
                    allowExpressions: true,
                    allowTypedFunctionExpressions: true,
                    allowHigherOrderFunctions: true,
                    allowDirectConstAssertionInArrowFunctions: true,
                },
            ],
            "@typescript-eslint/no-unused-vars": [
                "error",
                {
                    argsIgnorePattern: "^_",
                    varsIgnorePattern: "^_",
                    caughtErrorsIgnorePattern: "^_",
                },
            ],
            "@typescript-eslint/consistent-type-imports": [
                "error",
                {
                    prefer: "type-imports",
                    fixStyle: "inline-type-imports",
                },
            ],
            "@typescript-eslint/consistent-type-definitions": ["error", "interface"],
            "@typescript-eslint/no-require-imports": "error",

            "unicorn/filename-case": [
                "error",
                {
                    cases: {
                        kebabCase: true,
                        pascalCase: true,
                    },
                },
            ],
            "unicorn/no-null": "off",
            "unicorn/prevent-abbreviations": "off",
            "unicorn/no-array-reduce": "off",
            "unicorn/no-array-for-each": "off",
            "unicorn/prefer-module": "off",
            "unicorn/prefer-top-level-await": "off",

            "sonarjs/cognitive-complexity": ["warn", 15],
            "sonarjs/no-duplicate-string": ["warn", { threshold: 3 }],
            "sonarjs/no-identical-functions": "warn",
            "sonarjs/no-collapsible-if": "warn",
            "sonarjs/prefer-single-boolean-return": "warn",

            "no-console": ["warn", { allow: ["warn", "error"] }],
            "no-debugger": "error",
            "no-alert": "error",
            "prefer-const": "error",
            "no-var": "error",
            eqeqeq: ["error", "always"],
            curly: ["error", "all"],
        },
    },

    {
        files: ["**/main.ts"],
        rules: {
            "unicorn/prefer-top-level-await": "off",
        },
    },

    {
        files: ["apps/frontend/**/*.ts", "apps/frontend/**/*.tsx"],
        languageOptions: {
            globals: {
                ...globals.browser,
            },
        },
        rules: {
            "@typescript-eslint/explicit-function-return-type": "off",
        },
    },

    {
        files: ["apps/mobile/**/*.ts", "apps/mobile/**/*.tsx"],
        languageOptions: {
            globals: {
                ...globals.browser,
                __DEV__: "readonly",
            },
        },
        rules: {
            "@typescript-eslint/explicit-function-return-type": "off",
        },
    },

    {
        files: ["**/*.test.ts", "**/*.spec.ts", "**/tests/**/*.ts"],
        languageOptions: {
            globals: {
                ...globals.jest,
            },
        },
        rules: {
            "@typescript-eslint/no-explicit-any": "off",
            "sonarjs/no-duplicate-string": "off",
        },
    },

    {
        files: ["**/*.js", "**/*.mjs", "**/*.cjs"],
        ...tseslint.configs.disableTypeChecked,
        languageOptions: {
            globals: {
                ...globals.node,
            },
        },
    },
);
