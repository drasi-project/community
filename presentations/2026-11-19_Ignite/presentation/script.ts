import { scenes, timeLabel, duration, viewLabel } from "./scenes";
import { notes, actionLabels, glossary, questions, beatNumber, sentences } from "./notes";

// PRESENTER.md's run of show, script, glossary and questions, generated from notes.ts.
export function presenterScript() {
  const rows = scenes.map((scene, c) => {
    const views = [...new Set(notes[scene.id].beats.map(beat => viewLabel(beat.show)))].join(", ");
    const beats = `${beatNumber(c, 0)}–${beatNumber(c, notes[scene.id].beats.length - 1)}`;
    return `| ${timeLabel(scene.start)} | ${beats} | ${scene.title} | ${views} | ${notes[scene.id].summary} |`;
  });
  const run = ["| Time | Beats | Part | Projector shows | You |", "|---|---|---|---|---|", ...rows, "| 22:00 | | Questions | Session survey | Take questions |"].join("\n");
  const cues = scenes.map((scene, index) => {
    const cue = notes[scene.id];
    const beats = cue.beats.map((beat, b) => {
      const n = beatNumber(index, b) - 1;
      const label = `${viewLabel(beat.show)}${beat.optional ? " (optional)" : ""}`;
      // One sentence per line: a backslash at the end of a line is a Markdown line break.
      const lines = [`${n + 1}. **${label}${/[.?!]$/.test(label) ? "" : "."}**\\\n   ${sentences(beat.say).join("\\\n   ")}`];
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

The desk shows one beat at a time, numbered across the whole talk (the desk shows the same
numbers). Press **Next** (or your clicker, or →) to move on: the projector follows. Beats marked optional are skipped when **Short on time** is ticked.
Say it in your own words.

${cues}

## Glossary

${terms}

## Questions to expect

${answers}

<!-- end generated -->`;
}
