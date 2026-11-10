import type { ManualProgressionRequest, ProgressionView } from "../../../../../schemas/v1/progression-api";

export function inputError(money: string, points: string): string | null {
  if (money !== "" && (!/^(0|[1-9][0-9]{0,19})$/.test(money) || BigInt(money) > 18446744073709551615n)) {
    return "Money must be a whole copper amount from 0 to 18446744073709551615, without leading zeros.";
  }
  if (points !== "" && (!/^(0|[1-9][0-9]{0,4})$/.test(points) || Number(points) > 65535)) {
    return "Available talent points must be a whole number from 0 to 65535, without leading zeros.";
  }
  return money === "" && points === "" ? "Enter money or available talent points. Blank fields remain unknown." : null;
}

export function manualRequest(view: ProgressionView, money: string, points: string): ManualProgressionRequest {
  return {
    profile_id: view.profile_id, expected: view.binding, id: `manual-${crypto.randomUUID()}`,
    money: money === "" ? { status: "unknown" } : { status: "known", value: money },
    available_talent_points: points === "" ? { status: "unknown" } : { status: "known", value: Number(points) },
    spells: { status: "unknown" }, talents: { status: "unknown" }, professions: { status: "unknown" },
    recipes: { status: "unknown" }, bags: { status: "unknown" }, bank: { status: "unknown" }, equipment_enhancements: {},
  };
}

export function describeEnumeration(value: { completeness: "complete" | "partial"; entries: object }): string {
  const count = Object.keys(value.entries).length;
  return value.completeness === "complete"
    ? `${count} recorded entries; complete enumeration`
    : `${count} recorded entries; partial enumeration, other entries unknown`;
}
