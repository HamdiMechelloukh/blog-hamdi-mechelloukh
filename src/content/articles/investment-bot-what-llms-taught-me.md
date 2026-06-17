For two months, I tinkered together a small system that watches my portfolio and sends me, once a month, what it thinks I should do: buy, add, lighten, sell.

Wrong ideas, bugs hiding other bugs, decisions redone two or three times. And in the end, a much clearer picture of how language models actually behave, pretty far from what I imagined at the start.

## The bot in one sentence

Once a month, a small script runs on a server. It pulls the composition of my portfolio and public market data, asks an artificial intelligence to analyze all that, and sends me a Telegram message with its recommendations.

The rest of the article is how I got to this little automated-monitoring script, by using the power of an LLM "correctly".

## Act 1: the illusion of the edge

To be honest, I knew I couldn't have a head start on the market. But you still want to test the fantasy of code that makes you win on the stock market. I told myself that with enough data, a good model and a bit of code, I'd do as well as thousands of professional analysts.

It didn't last long. You can't beat the consensus with the consensus's own information; worse, you'll do worse than a good old "buy and hold on the S&P 500".

So I stopped chasing the edge. The value of this system was elsewhere: finding where the signals are favorable or not, seeing where I can't see, filtering out what I don't need to know, handing me proposals I could check, and then letting me make or not make a decision.

The edge exists, but it comes either from processing information better or faster than others (the whole quant business), or from having information others don't, and there it's either out of my reach, or it's insider trading. Me, scraping the same public numbers as everyone else, I have none.

The project took this realistic turn after that realization.

## Act 2: the parade of models

A bit of vocabulary first, because the whole article rests on it.

> **An LLM (Large Language Model)** is a program trained to predict the next word. You give it some text, it computes, for each possible word, a probability of being what comes next, then it picks one, and starts over. ChatGPT, Gemini, Claude are LLMs. That's all they do: predict the next word, one word at a time. The rest, the apparent reasoning, the analyses, emerges from this mechanism repeated billions of times.

My bot delegates its judgment to an LLM. Which one? I had to change the answer 6 times in 2 months, only to end up telling myself there isn't necessarily a right answer; you have to approach the LLM as a simple tool, like most SaaS.

Anyway, here's the path I took in terms of model choice:

```
Apr      Gemini alone
May 1    + Claude          (two models in parallel, to compare)
May 9    + Bear            (a 3rd model, deliberately pessimistic)
                           -> 3 voices, decision by vote
May 15   STOP. Opus only   (big cleanup, simplify everything)
May 22   back to Gemini    (cost and feature reasons)
Jun 13   MiMo              (Google terms-of-use change, and cost)
```

As you can see, at the start I began stacking models, because I had a serious lack of consistency in the bot's answers: one time it would tell me "buy Microsoft" and the next "sell Microsoft", even on 2 runs launched back to back.

It was pretty annoying, I was looking for a reliable answer, so the first idea was to reinforce the bot with 2 more LLM runs (Gemini) to get a kind of consensus, and I even went as far as adding a new model, Claude.

It was a bit better, but the bot was aggressive: it could recommend interesting names, but with too many negative theses. I needed a "devil's advocate", hence the idea of the "Bear", a 3rd LLM agent whose job was to look for the theses that lead to structural declines, and to cool down the "optimism" of the other two.

It was good, but it was expensive, and devilishly complex; something was off and it was tied to the architecture. I rewrote the bot, focusing and trying to simplify the prompts, and I still had the consistency problem.

After a few days, I went back to Gemini because my costs on Claude were a bit too high.

I started looking into the information the LLM was pulling in, and that's what made me remove the grounding search, because the LLM is heavily influenced by speculative noise.

In the end, I moved to MiMo: very good benchmark results, and a token usage cost (price per token + tokens needed to actually handle a task) that beats the competition. The terms-of-use changes for the Gemini API also cooled me off; when the free $300 disappear and you get a bill plus a withdrawal from your account worth 3 months of API usage, it kills the appetite.

## Act 3: the memory that made the bot paranoid

I had given the bot a memory. Concretely, a file that kept its past analyses over thirty days, fed back in on each new run. The idea seemed obvious: add context so the bot wouldn't answer randomly.

Except the memory created a bias. The model saw that it had suggested selling a position the week before, and that pushed it to confirm that sale, over and over, regardless of new facts. A past opinion became a conviction, with no regard for the fundamentals. I patched. I added a rule to remove sells from the memory. It moved the problem without solving it. I patched again. Then I found that the memory was producing an outright form of self-censorship: the model aligned with its past instead of looking at the present.

It's what I call the paradox of experience, a bit like a lot of older people: we lean on our experience to decide whether a choice is good or not, except the context of a situation changes, and so the same decision can become a good one in another context, something experience erases.

At some point, I counted the patches. When a feature needs fix upon fix and each fix calls for another, **the feature itself is the bug.** I removed the memory. The bot went back to being *stateless*: no state, no memory. Every month, it analyzes the situation fresh, as if discovering it.

Adding state (memory) to a system makes it more complex and introduces dependencies on the past that can corrupt the present. Statelessness is often a feature, not a lack. For those who remember their control-theory classes: by feeding the model's past outputs back into its input, I hadn't opened a loop, I had closed one, a positive feedback loop. The output reinforced itself, and the system diverged. Going back to stateless is precisely returning to an open loop, each run independent, with no feedback from the past.

Keep this memory story in mind. Part of the instability I blamed on it maybe wasn't its fault. We'll come back to it a bit later in the article.

## Act 4: speculation is not information

For the news monitoring, my first version used what's called *grounding* (or augmented retrieval).

> **Grounding** is when you let the LLM go fetch information from the web in real time while it answers, instead of relying only on what it memorized during training.

On paper, perfect: the model gets to read the latest news. In practice, it mostly brought back rumors, analyst speculation, "word is that...". Over a one-year horizon, that kind of information isn't information. It's noise dressed up as signal.

We're still facing a disturbance, this time at the input: the quality of the input signal itself.

About-face, again. I cut the grounding and built monitoring on verifiable sources only: official regulatory filings (in the US, the documents companies are legally required to publish), established financial news feeds, and for the rest of the world, targeted searches. Then I imposed two hierarchies on the collected information:

```
AUTHORITY RANKING               SEVERITY SCALE
official source      >          G3  structural (changes the thesis)
established newswire >          G2  notable
the rest                        G1  anecdotal
```

The goal was no longer to read everything, but to sort. An official filing announcing a change of leadership (G3, official source) doesn't weigh the same as a speculative opinion piece (G1, the rest). The noise filtering promised in Act 1 was taking shape.

## Act 5: the phantom facts

The system was now collecting verified facts. But the final recommendations seemed to ignore them. Worse: they were nearly identical to the runs where the monitoring had completely failed and brought back nothing at all. As if the facts didn't exist.

And yet they existed. They were right there in the text sent to the model. But they were grouped in a separate block, far from the place in the text where the model made its decision on each name. The model read them, then forgot them when it came time to decide.

The fix, once the diagnosis was made, was dumb: place each fact right next to the name it concerns, at the exact moment the model rules on that name. The lesson, though, runs deep:

**With an LLM, the position of a piece of information matters as much as its presence.** A fact present in the context but badly placed relative to the decision point is, in practice, an absent fact, especially when the context is long.

A corollary I wrote into the code right after: **if the facts layer fails, the system crashes.** It doesn't send a degraded report. A plausible but hollow report is more dangerous than no report, because it looks like real analysis. Better a visible crash than a false certainty nicely dressed up.

## Act 6: the principle that reorganized everything

By dint of correcting behavioral drifts, I ended up formulating the rule that underpins everything else, and which is probably my biggest realization of the project.

**The instructions given to an LLM must be principles, never numerical rules. Determinism must live in the data, not in the text of the instruction.**

> **Deterministic** means: always gives the same result from the same inputs. A computation is deterministic. Human judgment is not.

Concretely, from then on I forbade myself from writing things in the instructions like "aim for about 20% on this position" or "only do this". Why? Because a numerical threshold written in natural language gives you the worst of both worlds:

- it makes the model **rigid** where I wanted nuanced judgment;
- and it hands it a number to **cling to** and to **make things up** around (LLMs have an annoying tendency to embroider around the numbers you give them, because their answer is an estimate of the best answer to give, not the best answer to give).

If I want determinism, it has to be upstream, in the pipe that prepares the data. The mental model became this:

```
   UPSTREAM: DETERMINISTIC           DOWNSTREAM: NON-DETERMINISTIC
   (code, numbers)                   (the LLM's judgment)
 ┌────────────────────────────┐    ┌──────────────────────────┐
 │ portfolio at market value  │    │ weighs the pros and cons │
 │ universe filtered & scored │ ──> │ arbitrates between names │
 │ verified, dated facts      │    │ writes target weights    │
 │ consensus reliability      │    │ following PRINCIPLES      │
 └────────────────────────────┘    └──────────────────────────┘
   the numbers constrain              no magic number
   (computed, verifiable)             in the instructions
```

Everything that can be computed cleanly is computed upstream, in code, in a verifiable way, and handed to the model as a numerical constraint. The model, for its part, receives principles ("favor strong convictions", "a reliable consensus beats a high but uncertain target") and judges.

One last rule in the same spirit, on writing instructions: **one rule = one statement, said once.** Two near-identical rules are worse than one, because the reader (and an LLM even more so) looks for the difference between them. Since it doesn't exist, it invents one. Rewriting to condense isn't cosmetic, it's reducing the surface for error.

## Act 7: the discovery, the noise was there from the start

Then comes the move to MiMo. And with it, not a new problem, but the revelation of an old problem I had never seen. To understand it, three definitions, like a staircase.

> **1. The probability distribution.** At each word, the LLM doesn't pick "the" next word. It computes a probability for each possible word. For instance, after "the cat drinks", it might rate: *milk* 70%, *water* 25%, *coffee* 4%, etc.

> **2. The temperature.** It's the knob that sets the randomness of the selection. High temperature: the model sometimes picks unlikely options (more creative, more unpredictable). Zero temperature: it systematically takes the most likely option. *milk*, every time.

> **3. The logits.** These are the raw scores the model computes for each word before turning them into probabilities. They're the raw material of the decision.

At zero temperature, the model always takes the most likely word, so with the same inputs it should produce exactly the same output. Deterministic. In reality, I had misunderstood what an LLM is. It gives the illusion of an exact answer, when in fact it mostly gives an answer that's "good enough". When you code with an LLM, for example, it spits out working code, but not necessarily good code, or rather: not every time.

When I changed models, for the first time I wanted to measure stability before building on it. I ran **the same prompt, on the same frozen data, several times in a row, at zero temperature.** I expected identical outputs.

I got the opposite. Considerable variance from one run to the next. A few real numbers from this test on about thirty positions:

- the sum of the proposed target allocations came to **44%** on the first run, **105%** on the second, **101%** on the third;
- out of the thirty-odd names, **only 6** got a stable recommendation from one run to the next;
- one run in three went completely off the rails.

Same input, same zero temperature, very different outputs.

**This noise was not new with MiMo.** It had been there from the start, with Gemini, then Claude, then every model I had used. I tried to fix it with deterministic instructions, because I needed control, but that's nonsense: I can't have both control (deterministic) and the AI finding me insights (non-deterministic). MiMo didn't bring anything special on this front; it was just the occasion where I understood the line between using the LLM or not, because it's just a tool, with, admittedly, a high level of abstraction, but not a solution or a human replacement, even if some companies do very nice marketing to say otherwise.

And there, Act 3 takes on another meaning. The instability I had blamed on the memory, those recommendations that flip-flopped from month to month that I fought with patches, a good chunk of it probably wasn't the memory at all. It was already this noise, invisible for lack of being measured.

Careful, though, not to rewrite history too cleanly: the memory's anchoring bias was real, feeding back its own past sells creates a genuine confirmation bias. So there were two overlapping problems, not a single misdiagnosed one. The memory wasn't innocent; it simply had an invisible accomplice I wasn't measuring.

### Why an LLM stays noisy even at zero temperature

Here's the heart of it, and it's subtler than it looks.

At zero temperature, the choice of word is **not** random. The model takes the maximum, perfectly deterministically. **So it's not the selection rule that injects randomness.**

The noise comes one notch earlier: **the logits themselves don't always land on the same number.** And the cause isn't the one people assume. The common explanation ("the parallel computations on the GPU run in a random order") is misleading, and that's exactly what the work cited just below corrects: for a given computation shape, the model is in fact reproducible; re-run identically, it gives back the same logits.

The real culprit is the **batch**. On a server, your request is never handled alone: it's grouped with others, and the composition of that group (how many requests, of what lengths) changes on every call depending on load. And to go fast, the GPU splits up and adds the numbers in an order that depends on the shape of the batch. And floating-point addition (the computer's approximate arithmetic on decimal numbers) isn't associative: `(a + b) + c` doesn't give exactly `a + (b + c)`. So different batch neighbors lead to a different split, hence a different order of additions, hence logits that move by a hair. Each computation taken in isolation is deterministic; it's the batch context that varies from one call to the next.

This isn't an absolute fatality, by the way. [Recent work by Thinking Machines Lab](https://thinkingmachines.ai/blog/defeating-nondeterminism-in-llm-inference/) showed that by rewriting these computations so they always add in the same order, whatever the batch composition, you can make a model perfectly reproducible at zero temperature: in their demo, 1000 generations became bit-for-bit identical, where the standard version produced 80 different ones. The price is a slowdown (on the order of 1.6 to 2 times depending on the kernels), and that's why consumer APIs don't enable it by default. So the noise at zero temperature is a fatality in practice, on common APIs, not in principle.

As long as the best candidate wins comfortably, it doesn't matter. But when two candidates are nearly tied, that hair flips the ranking:

```
Raw scores (logits) for the next word

  lighten   ████████████████████  8.40
  sell      ███████████████████▉  8.39   <- nearly tied!
  hold      ██████████            4.10

Run 1:  lighten 8.401 , sell 8.399  ->  we pick LIGHTEN
Run 2:  lighten 8.398 , sell 8.402  ->  we pick SELL
                       ^ tiny floating-point gap, and it all flips
```

And on a model that "reasons" (that generates a long chain of thought before concluding), a single early flip propagates and amplifies all along the reasoning. A hair's difference at the start, an opposite conclusion at the end.

So the right way to put it is:

> It's not noisy because the choice is random, but because **the estimated values that a deterministic choice rests on are themselves unstable.** A deterministic choice over unstable estimates becomes unstable again near the ties.

And that detail changes the whole interpretation. **The noise isn't blind.** It concentrates exactly on the close calls, the ones where the model itself is undecided. A position where the conviction is clear never flips (the best candidate wins comfortably). The positions that waltz from one run to the next are precisely the ones where two options are nearly equal.

**The noise marks the zones of real uncertainty.** It's not a flaw to hide. It's information.

## Act 8: don't fight the noise, make it vote

If the noise is information about uncertainty, the right response isn't to eliminate it. It's to aggregate it.

Rather than a single run, I launch several on the same data, then I make the results vote. It's exactly the idea of the **Condorcet jury theorem**, stated by an 18th-century French mathematician.

> **Condorcet's theorem (in a few words).** If each juror has a better-than-50% probability of finding the right answer, and the jurors err independently of one another, then the more jurors you add, the more the majority vote tends toward the certainty of being right.

As a formula, the probability that the majority of N jurors is right, each correct with probability p:

```
                N
P(majority) =   Σ    C(N,k) · p^k · (1 − p)^(N−k)
              k=⌊N/2⌋+1
```

What to take from it without the symbols:

```
  p (juror quality)            majority vote as N grows
  ─────────────────            ───────────────────────────────────────
  p > 0.5  (better than coin)  ──> tends to 1   (certain to be right)
  p = 0.5  (coin flip)         ──> stays at 0.5 (voting doesn't help)
  p < 0.5  (worse than coin)   ──> tends to 0   (voting makes it worse!)
```

Watch the trap the formula makes visible: voting only improves things **if each juror is already better than chance.** If the model is bad on a question, multiplying the runs only amplifies the error. Voting makes a correct-but-noisy juror reliable; it doesn't save an incompetent one.

And there's a second trap, more insidious. Condorcet's theorem has **two** assumptions, not one: jurors better than chance (I just talked about that), and **independent** errors. But re-running the same model five times is five times the same network, the same biases, the same typical reasoning. The floating-point noise only decorrelates the outputs near the ties, exactly where I want them to vote. But on a systematic error (the model doesn't understand a sector, overrates a thesis), the five runs are wrong together, and worse: they're wrong **unanimously**. Because a unanimous vote is only a **constant answer**, and a constant answer is only the **reinforcement of the model's thesis**, not proof that it's right. 5/5 measures stability, never truth. To settle a consensus, you therefore need a source **outside the model**; the same model re-run will only repeat its thesis with confidence. Voting neutralizes the sampling noise; it doesn't correct the model's bias.

My model, on most positions, is far better than chance, just noisy near the close calls. An ideal use case for Condorcet. I refined the vote on two levels: first the **direction** (should this position go up or down?), then only the **degree**. Without that, a clear consensus on direction could get buried under slightly different action labels ("lighten" and "sell" both say: go down).

The result is exactly what I'd been looking for since Act 1 without knowing it:

```
  Stock A  sell 5/5      ->  stable: top of the map (to validate against facts)
  Stock B  add 4/5       ->  stable: consensus
  Stock C  buy 2 / lighten 2 / hold 1
                         ->  NO consensus: shown as "split,
                             your judgment decides"
```

The clear cases come out by consensus, the close ones show up **honestly** as split, and I'm the one who decides. But careful not to read this table as a ranking of good recommendations: after everything above, a "5/5" doesn't certify that selling is the right move, it certifies that the model is **stable** on it. What the ensemble really produces isn't a list of orders, it's a **stability map**: here's where my judgment is least needed, and here's where it's needed most. The consensus tells me where to look with confidence; it's the primary facts and me who validate the substance. The system stops pretending to a certainty it doesn't have.

**I had started by making three different models vote, then I deleted everything to simplify. I end up making a single model vote several times.** The structure is the same (a vote), but the reason changed completely: at the start I voted to combine different viewpoints; in the end I vote to neutralize the noise of a single model. It took me two months and a detour through the whole chain to understand the real point of that vote.

Then there's the objection: the version truly faithful to Condorcet would be several different models, each run several times, because different architectures err in a more decorrelated way. And let's be honest: the gain is **real**, not uncertain. An ensemble of different models really does decorrelate errors, that's been the whole point of ensembles forever. It's simply probably not **worth it**. Each extra model is one more provider to maintain, to pay, to monitor, formats and prompts to keep in sync, for a benefit that becomes marginal next to that operating cost. You leave Pareto's useful 80%. It's a cost trade-off, not a denial of the benefit. So I decided otherwise. The vote of a single model kills the noise, which is the essential part and nearly free. And for the bias, the source outside the model that I need, I already have it: the primary facts I confront the model with (Acts 4 and 5), and my own judgment on the split cases. The 5/5 tells me where the model is stable; the facts and I say whether it's right.

## The question this article dodges

Everything above is about **consistency**: is the system stable, honest about its doubts? But consistency isn't **correctness**. A system can be perfectly stable, perfectly clear-eyed about its zones of uncertainty, and mediocre in returns. Perfect consistency is even, as we just saw, exactly what a prejudice repeated without flinching looks like.

I myself dismantled the idea of an edge back in Act 1: I have no serious reason to beat the market. So a tension remains that I don't really resolve: what good are such carefully crafted allocations if nothing guarantees they're better than a simple index fund? My honest answer: it's not a performance tool, it's a monitoring and decision-support tool. The targets it produces are a starting point for my judgment, not autopilot. It's still too early to measure whether I beat an index over time. For now, I'm very close to the S&P 500 and below the NASDAQ, over 2-3 months of development, which proves nothing, one way or the other: over that span, everything is drowned in market noise. Telling skill from luck in equity allocation takes years, and ideally going through a real downturn. So I won't have the answer for a long time, now that the bot is stable.

I wrote this article to explain how I made a machine consistent in its answers and how I gained lucidity about its limits, without claiming that it's right.

## What I was really looking for

I started out wanting a machine that's right. I ended up with a machine that's honest about what it doesn't know. And that's far more useful.

After dismantling my illusions one by one, here's what remains, and what's enough to justify the machine: a monitoring system, with a reliable sieve against the noise of information, to surface what matters on the market and in my portfolio. No promise of beating the market. But at the one task I built it for, gathering the right information and discarding the rest, it is, without hesitation, better than me.

The most transferable lesson reaches far beyond finance:

> **An LLM isn't an oracle, it's a sampler.** It draws its answers from a probability distribution. Its variance isn't a flaw to hide, it's a measure of its own confidence. Good systems built around LLMs don't hide uncertainty, they bring it to the surface.

The bot runs once a month and sends me its conclusions. But what it really gave me isn't a shopping list. It's a sharper way to think about decisions under uncertainty, and a deep respect for the difference between a system that answers, and a system that knows when it doesn't know.
