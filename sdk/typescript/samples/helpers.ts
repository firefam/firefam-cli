import path from "node:path";

export function firefamPathOverride() {
  return (
    process.env.FIREFAM_EXECUTABLE ??
    path.join(process.cwd(), "..", "..", "firefam-rs", "target", "debug", "firefam")
  );
}
