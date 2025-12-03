#!/usr/bin/env node

import { createInterface } from "node:readline";

const WASM_DIR = process.argv[2];

const reader = createInterface({ input: process.stdin });

let WASM_PATH = `${WASM_DIR}/__NULL__.wasm`;

const encodeAction = (action) => {
  switch (action.type) {
    case "invoke": {
      const invokeFn = action.field;
      const invokeArgs = action.args.map(({ value }) => value).join(" ");
      return `${WASM_PATH} --invoke ${invokeFn} ${invokeArgs}`;
    }
    case "get": {
      const getField = action.field;
      return `${WASM_PATH} --get ${getField}`;
    }
  }
};

const encodeCommand = (command) => {
  switch (command.type) {
    case "module":
      WASM_PATH = `${WASM_DIR}/${command.filename}`;
      break;
    case "action": {
      return encodeAction(command.action);
    }
    case "assert_return": {
      const returnArgs = command.expected.map(({ value }) => value).join(" ");
      return `${encodeAction(command.action)} --assert-return ${returnArgs}`;
    }
    case "assert_trap": {
      return `${encodeAction(command.action)} --assert-trap`;
    }
    case "assert_invalid":
      return `${WASM_DIR}/${command.filename} --assert-invalid`;
    case "assert_malformed":
      return `${WASM_DIR}/${command.filename} --assert-malformed`;
    default: {
      console.error(JSON.stringify(command));
    }
  }
};

let acc = "";

const safeParse = (str) => {
  try {
    const parsed = JSON.parse(acc + str);
    acc = "";
    return parsed;
  } catch (e) {
    acc += str;
  }
};

for await (const line of reader) {
  const command = safeParse(line);
  if (!command) continue;
  const encoded = encodeCommand(command);
  if (encoded) {
    console.log(encoded);
  }
}
