// Only diagnostic instrumentation uses this deadline; never wraps parity APIs.
export async function diagnosticDeadline(promise, operation) {
  let timer;
  try {
    return await Promise.race([promise, new Promise((_, reject) => {
      timer = setTimeout(() => reject(new Error(`Diagnostic ${operation} exceeded 10000ms`)), 10000);
    })]);
  } finally { clearTimeout(timer); }
}
