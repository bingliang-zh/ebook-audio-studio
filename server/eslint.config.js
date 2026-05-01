import js from "@eslint/js";
import globals from "globals";

export default [
  { ignores: ["dist", "storage"] },
  {
    files: ["**/*.ts"],
    languageOptions: {
      ecmaVersion: 2022,
      globals: globals.node
    },
    rules: js.configs.recommended.rules
  }
];
