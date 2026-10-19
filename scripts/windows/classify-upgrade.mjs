// Shared classification entrypoint for the PowerShell packaging workflow.
import { classifyUpgrade } from "./native-smoke-lib.mjs";
if (process.argv.length !== 4) throw new Error("Expected baseline and candidate SemVer versions");
console.log(JSON.stringify(classifyUpgrade(process.argv[2], process.argv[3])));
