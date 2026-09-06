import { version as packageVersion } from '../../package.json';

/** App version; package.json is the source of truth (Cargo.toml and tauri.conf.json must match). */
export const version: string = packageVersion;
