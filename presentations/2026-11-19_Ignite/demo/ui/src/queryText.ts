import overdue from "../../drasi/overdue-tasks.cypher?raw";
import freshness from "../../drasi/endpoint-freshness.cypher?raw";
import decision from "../../drasi/decision-context.cypher?raw";
import result from "../../drasi/decision-results.cypher?raw";
import model from "../../drasi/model-context.cypher?raw";
import task from "../../drasi/task-context.cypher?raw";
import attempt from "../../drasi/attempt-context.cypher?raw";
import breach from "../../drasi/breach-history.cypher?raw";
import alerts from "../../drasi/alert-context.cypher?raw";
import recovery from "../../drasi/recovery-results.cypher?raw";
import recoveryTasks from "../../drasi/recovery-task-context.cypher?raw";
import recoveryObservations from "../../drasi/recovery-observation-context.cypher?raw";
import rego from "../../policies/placement.rego?raw";

export const queryText: Record<string, string> = {
  "overdue-tasks": overdue, "endpoint-freshness": freshness, "decision-context": decision,
  "policy-results": result, "allocation-results": result, "resilience-results": result,
  "model-context": model, "task-context": task, "attempt-context": attempt, "breach-history": breach,
  "alert-context": alerts,
  "recovery-results": recovery,
  "recovery-task-context": recoveryTasks, "recovery-observation-context": recoveryObservations,
};
export { rego };
