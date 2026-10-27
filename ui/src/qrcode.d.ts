declare module "qrcode" {
  export function toDataURL(
    text: string,
    options?: object,
  ): Promise<string>;
  const _default: { toDataURL: typeof toDataURL };
  export default _default;
}
