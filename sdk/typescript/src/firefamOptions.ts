export type FirefamConfigValue = string | number | boolean | FirefamConfigValue[] | FirefamConfigObject;

export type FirefamConfigObject = { [key: string]: FirefamConfigValue };

export type FirefamOptions = {
  firefamPathOverride?: string;
  baseUrl?: string;
  apiKey?: string;
  /**
   * Additional `--config key=value` overrides to pass to the Firefam CLI.
   *
   * Provide a JSON object and the SDK will flatten it into dotted paths and
   * serialize values as TOML literals so they are compatible with the CLI's
   * `--config` parsing.
   */
  config?: FirefamConfigObject;
  /**
   * Environment variables passed to the Firefam CLI process. When provided, the SDK
   * will not inherit variables from `process.env`.
   */
  env?: Record<string, string>;
};
