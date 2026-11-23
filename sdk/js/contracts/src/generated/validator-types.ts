/** Minimal Ajv error shape retained by standalone validators. */
export interface ErrorObject { readonly instancePath: string; readonly schemaPath: string; readonly keyword: string; readonly params: Readonly<Record<string, unknown>>; readonly message?: string }
/** Runtime shape shared by generated standalone Ajv validators. */
export interface ValidateFunction { (value: unknown): boolean; errors?: readonly ErrorObject[] | null }
