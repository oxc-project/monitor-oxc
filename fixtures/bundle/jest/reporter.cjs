class DeterministicReporter {
  onTestResult(test, result) {
    for (const t of [...result.testResults].sort((a, b) => a.fullName.localeCompare(b.fullName))) {
      console.log(`${t.fullName}: ${t.status}`);
    }
  }
  onRunComplete(_, results) {
    console.log(`suites=${results.numTotalTestSuites} tests=${results.numTotalTests} failed=${results.numFailedTests}`);
  }
}

module.exports = DeterministicReporter;
