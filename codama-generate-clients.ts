import { renderVisitor as renderJavaScriptVisitor } from "@codama/renderers-js";
import { renderVisitor as renderRustVisitor } from "@codama/renderers-rust";
import {
  argumentValueNode,
  arrayTypeNode,
  createFromRoot,
  instructionRemainingAccountsNode,
  numberTypeNode,
  prefixedCountNode,
  publicKeyTypeNode,
  tupleTypeNode,
  updateInstructionsVisitor,
} from "codama";
import { setFixedAccountSizesVisitor } from "@codama/visitors";
import fs from "fs";
import path from "path";

const rustClientsDir = path.join(__dirname, "..", "sdk", "rust");
const typescriptClientsDir = path.join(__dirname, "..", "sdk", "ts");

const codama = createFromRoot(
  require(path.join(__dirname, "idl", "token_acl_gate_program.json")),
);

function preserveConfigFiles() {
  const filesToPreserve = [
    "package.json",
    "tsconfig.json",
    ".npmignore",
    "pnpm-lock.yaml",
    "Cargo.toml",
  ];
  const preservedFiles = new Map();

  filesToPreserve.forEach(filename => {
    const filePath = path.join(typescriptClientsDir, filename);
    const tempPath = path.join(typescriptClientsDir, `${filename}.temp`);

    if (fs.existsSync(filePath)) {
      fs.copyFileSync(filePath, tempPath);
      preservedFiles.set(filename, tempPath);
    }
  });

  return {
    restore: () => {
      preservedFiles.forEach((tempPath, filename) => {
        const filePath = path.join(typescriptClientsDir, filename);
        if (fs.existsSync(tempPath)) {
          fs.copyFileSync(tempPath, filePath);
          fs.unlinkSync(tempPath);
        }
      });
    }
  };
}

const configPreserver = preserveConfigFiles();

codama.accept(renderJavaScriptVisitor("sdk/ts/src/generated", { formatCode: true,  }));
codama.accept(renderRustVisitor("sdk/rust/src/generated", { crateFolder: "sdk/rust/", formatCode: true }));
