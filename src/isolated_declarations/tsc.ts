import * as ts from 'typescript'
import { diff } from 'jest-diff';
import path from "node:path"
import { readFileSync } from 'node:fs'

function transformByTypeScript(filePath: string, code: string) {
  let result = ts.transpileDeclaration(code, { fileName: filePath })
  if (result.diagnostics && result.diagnostics.length) {
    throw new Error(result.diagnostics)
  }
  return codegen(filePath, result.outputText)
}

function codegen(filePath: string, sourceText: string) {
  let sourceFile = ts.createSourceFile(
    filePath,
    sourceText,
    ts.ScriptTarget.Latest,
  )
  return ts.createPrinter({ removeComments: true }).printFile(sourceFile)
}


function compare() {
  const dirname = __dirname;
  let filepath = path.join(dirname, "input.ts")
  let code = readFileSync(filepath).toString();
  let output = transformByTypeScript(filepath, code)
  let outputPath = path.join(dirname, "output.d.ts")
  let expected = codegen(outputPath, readFileSync(outputPath).toString())
  if (output !== expected) {
    console.log(diff(output, expected))
  }
}

compare()