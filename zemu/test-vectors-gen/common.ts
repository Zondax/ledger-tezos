export const MUTEZ_MULT = 1_000_000

export const RPC_ADDR = "https://granadanet.api.tez.ie/"

// TODO: format string like we want to format it in the ledger
// do paging and also number formatting
export function ledger_fmt(input: string): string[] {
    return [input];
}

// Do number formatting (and paging)
export function ledger_fmt_currency(input: string): string[] {
    const num = Number.parseInt(input) / MUTEZ_MULT;

    const fp = num.toFixed(6);

    return [fp];
}
