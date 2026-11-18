import { scenes, timeLabel, duration, viewLabel } from "./scenes";
import { notes, actionLabels, glossary, questions } from "./notes";

// PRESENTER.md's run of show, script, glossary and questions, generated from notes.ts.
export function presenterScript() {
  const rows = scenes.map(scene => {
    const views = [...new Set(notes[scene.id].beats.map(beat => viewLabel(beat.show)))].join(", ");
    return `| ${timeLabel(scene.start)} | ${scene.title} | ${views} | ${notes[scene.id].summary} |`;
  });
  const run = ["| Time | Part | Projector shows | You |", "|---|---|---|---|", ...rows, "| 22:00 | Questions | Session survey | Take questions |"].join("\n");
  const cues = scenes.map((scene, index) => {
    const cue = notes[scene.id];
    const beats = cue.beats.map((beat, n) => {
      const lines = [`${n + 1}. **${viewLabel(beat.show)}${beat.optional ? " (optional)" : ""}.** ${beat.say}`];
      if (beat.click) lines.push(`   - **Click on the projector:** ${beat.click}${beat.expect === "rejected" ? " (it's meant to be rejected)" : ""}. ${beat.do?.includes("approve-change") ? ` Backup on the desk: ${actionLabels["approve-change"]} (it presses the same button)` : ""}`);
      else if (beat.do) lines.push(`   - **Press:** ${beat.do.map(id => actionLabels[id]).join(", then ")}${beat.expect === "rejected" ? " (it's meant to be rejected)" : ""}`);
      if (beat.watch) lines.push(`   - **Watch for:** ${beat.watch}`);
      return lines.join("\n");
    }).join("\n");
    return [`### ${index + 1}. ${scene.title} (${duration(scene.minutes)})`, "", beats, "",
      `**If it breaks:** ${cue.ifBroken}`].join("\n");
  }).join("\n\n");
  const terms = ["| Term | Meaning |", "|---|---|", ...glossary.map(([term, meaning]) => `| ${term} | ${meaning} |`)].join("\n");
  const answers = ["| Question | Answer |", "|---|---|", ...questions.map(([q, a]) => `| ${q} | ${a} |`)].join("\n");
  return `<!-- generated from presentation/notes.ts: do not edit; run npm run sync-presenter in demo/ui -->

## Run of show

${run}

## The script

The desk shows one beat at a time. Press **Next** (or your clicker, or →) to move on: the
projector follows. Beats marked optional are skipped when **Short on time** is ticked.
Say it in your own words.

${cues}

## Glossary

${terms}

## Questions to expect

${answers}

<!-- end generated -->`;
}
