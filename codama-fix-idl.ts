import { renderVisitor as renderJavaScriptVisitor } from "@codama/renderers-js";
import { renderVisitor as renderRustVisitor } from "@codama/renderers-rust";
import {
  argumentValueNode,
  arrayTypeNode,
  bytesTypeNode,
  bytesValueNode,
  createFromRoot,
  fixedSizeTypeNode,
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
  require(path.join(__dirname, "target", "idl", "token_acl_gate_program.json")),
);

codama.update(
  updateInstructionsVisitor({
    setupExtraMetas: {
      arguments: {
        addresses: {
          type: arrayTypeNode(
            publicKeyTypeNode(),
            prefixedCountNode(numberTypeNode("u32")),
          ),
          docs: ["Up to 5 ListConfig accounts owned by this program"],
        },
      },
      remainingAccounts: [
        instructionRemainingAccountsNode(
          argumentValueNode("addresses"),
          { isSigner: false, isWritable: false },
        ),
      ],
    },
    setupFreezeExtraMetas: {
      arguments: {
        addresses: {
          type: arrayTypeNode(
            publicKeyTypeNode(),
            prefixedCountNode(numberTypeNode("u32")),
          ),
          docs: ["Up to 5 ListConfig accounts owned by this program"],
        },
      },
      remainingAccounts: [
        instructionRemainingAccountsNode(
          argumentValueNode("addresses"),
          { isSigner: false, isWritable: false },
        ),
      ],
    },
    canThawPermissionless: {
      arguments: {
        // the program dispatches on the first byte but requires the complete
        // 8-byte token-acl interface discriminator:
        // sha256("efficient-allow-block-list-standard:can-thaw-permissionless")[..8]
        discriminator: {
          type: fixedSizeTypeNode(bytesTypeNode(), 8),
          defaultValue: bytesValueNode('base16', '08afa981894a3df1'),
        },
        addresses: {
          type: arrayTypeNode(
            publicKeyTypeNode(),
            prefixedCountNode(numberTypeNode('u32'))
          ),
          docs: ['Pairs of (ListConfig, WalletEntry) accounts. Must be passed as a flattened array.'],
        },
      },
      remainingAccounts: [
        instructionRemainingAccountsNode(
          argumentValueNode('addresses'),
          { isSigner: false, isWritable: false }
        ),
      ],
    },
    canFreezePermissionless: {
      arguments: {
        // the program dispatches on the first byte but requires the complete
        // 8-byte token-acl interface discriminator:
        // sha256("efficient-allow-block-list-standard:can-freeze-permissionless")[..8]
        discriminator: {
          type: fixedSizeTypeNode(bytesTypeNode(), 8),
          defaultValue: bytesValueNode('base16', 'd68d6d4bf8012d1d'),
        },
        addresses: {
          type: arrayTypeNode(
            publicKeyTypeNode(),
            prefixedCountNode(numberTypeNode('u32'))
          ),
          docs: ['Pairs of (ListConfig, WalletEntry) accounts. Must be passed as a flattened array.'],
        },
      },
      remainingAccounts: [
        instructionRemainingAccountsNode(
          argumentValueNode('addresses'),
          { isSigner: false, isWritable: false }
        ),
      ],
    },
  }),
);

codama.update(setFixedAccountSizesVisitor());

const updatedIdl = codama.getJson();
const formattedIdl = JSON.stringify(JSON.parse(updatedIdl), null, 2);

// Write to a temp file and rename so the committed IDL is never left
// truncated if the write fails partway through.
const idlPath = path.join(__dirname, "idl", "token_acl_gate_program.json");
const tmpPath = `${idlPath}.tmp`;
fs.writeFileSync(tmpPath, formattedIdl);
fs.renameSync(tmpPath, idlPath);
