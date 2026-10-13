import type { DataContext } from "../../../../schemas/v1/exchange";
export interface CalibrationConditions { route_stage: string; level: number; role: string; party_size: number }
export interface SessionCalibrationAnnotation { conditions: CalibrationConditions; complete: boolean; completion_quest: string }
export interface CalibrationRequest { conditions: CalibrationConditions; baseline_ms: number; baseline_source: string }
export interface CalibrationReport {
  methodology_version: number; character: string; context: DataContext; fixture: boolean; request: CalibrationRequest;
  sample_count: number; median_ms: number | null; minimum_ms: number | null; maximum_ms: number | null;
  blended_ms: number; uncertainty: string;
  comparisons: {session_id: string; annotation: SessionCalibrationAnnotation | null; observed_ms: number | null; difference_ms: number | null; exclusion: string | null; outlier: boolean}[];
  methodology: string;
}
