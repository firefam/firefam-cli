import fs from "node:fs/promises";
import os from "node:os";
import path from "node:path";

import { afterEach, beforeEach } from "@jest/globals";

const originalFirefamHome = process.env.FIREFAM_HOME;
let currentFirefamHome: string | undefined;

beforeEach(async () => {
  currentFirefamHome = await fs.mkdtemp(path.join(os.tmpdir(), "firefam-sdk-test-"));
  process.env.FIREFAM_HOME = currentFirefamHome;
});

afterEach(async () => {
  const firefamHomeToDelete = currentFirefamHome;
  currentFirefamHome = undefined;

  if (originalFirefamHome === undefined) {
    delete process.env.FIREFAM_HOME;
  } else {
    process.env.FIREFAM_HOME = originalFirefamHome;
  }

  if (firefamHomeToDelete) {
    await fs.rm(firefamHomeToDelete, { recursive: true, force: true });
  }
});
