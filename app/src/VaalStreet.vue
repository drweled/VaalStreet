<template>
  <div class="vs-page" :class="{ 'vs-page-signin': !ready }" :style="{ fontSize: `${fontPx}px` }" @mousedown.self="hideWindow">
    <div ref="rootEl" class="ww-root wf-frame" :class="{ 'ww-small': !ready }">
      <!-- not signed in: only the logo and the one button -->
      <div v-if="!ready" class="ww-signin" @mousedown="dragWindow">
        <button class="wf-tool wf-close ww-tool vs-close vs-close-corner" title="Close (Esc)" @mousedown.stop @click="hideWindow"><i class="fas fa-times" /></button>
        <div class="ww-big-logo wf-wordmark">VAAL STREET</div>
        <button v-if="checking" class="wf-primary ww-btn ww-signin-btn" disabled @mousedown.stop>
          <i class="fas fa-circle-notch fa-spin" /> Checking sign-in…
        </button>
        <button v-else-if="waitingSignIn" class="wf-primary ww-btn ww-signin-btn" @mousedown.stop @click="signIn">
          <i class="fas fa-circle-notch fa-spin" /> Waiting for the sign-in window…
        </button>
        <button v-else class="wf-primary ww-btn ww-signin-btn" @mousedown.stop @click="signIn">
          <i class="fas fa-sign-in-alt" /> Sign in
        </button>
        <div class="vs-made-by">made by
          <a class="vs-maker" :href="MAKER_PROFILE" title="Waleed's Path of Exile profile"
            @click.prevent="openMaker">Waleed</a>
        </div>
      </div>

      <!-- title bar -->
      <div v-if="ready" class="ww-head vs-head" @mousedown="dragWindow">
        <div class="ww-brand">
          <div class="ww-logo wf-wordmark">VAAL STREET</div>
          <div class="wf-tagline ww-tag">stash · worth · loot</div>
        </div>
        <div class="ww-sub">
          <span class="ww-acc"><i class="fas fa-user" /> {{ account }}</span>
          <!-- no price checker here: the league is picked in the title bar -->
          <select v-model="leagueChoice" class="ww-select vs-league" title="League (stash and poe.ninja prices)">
            <option v-for="l in leagues.list.value" :key="l.id" :value="l.id">{{ l.id }}</option>
          </select>
          <span v-if="syncedAgo" class="ww-last" :class="{ old: Date.now() - lastSyncAt > 3_600_000 }" :title="`Last synced ${fmtLong(lastSyncAt)} · press Sync for your wealth now`">
            <i class="fas fa-history" /> last synced {{ syncedAgo === 'just now' ? 'just now' : `${syncedAgo} ago` }}
          </span>
          <span v-else-if="selectedIds.length" class="ww-last old">not synced yet</span>
          <button class="wf-primary ww-btn ww-sync" :disabled="busy || !selectedIds.length" :title="busy ? `${progressText} (pathofexile.com limits how fast stash tabs can be read: waiting keeps you from being timed out)` : ''" @mousedown.stop @click="sync()">
            <i class="fas fa-sync-alt" :class="{ 'fa-spin': busy }" /> {{ busy ? progressText : 'Sync' }}
          </button>
          <!-- a newer release on GitHub -->
          <button v-if="update" class="vs-update" :disabled="updating"
            :title="update.notes || `Vaal Street ${update.version} is out (you have ${update.current})`"
            @click="installUpdate">
            <i class="fas" :class="updating ? 'fa-circle-notch fa-spin' : 'fa-download'" />
            {{ updating ? 'Updating…' : `Update to ${update.version}` }}
          </button>
          <!-- start Vaal Street with Windows, or not -->
          <button class="vs-startup" :class="{ on: startupOn }" :disabled="startupBusy"
            :title="startupOn ? 'Vaal Street starts with Windows: click to stop that' : 'Click to start Vaal Street with Windows'"
            @click="toggleStartup">
            <i class="fas fa-power-off" /> Startup <b>{{ startupOn ? 'ON' : 'OFF' }}</b>
          </button>
          <span class="ww-toolset">
          <button class="wf-tool ww-tool" :title="theme === 'dark' ? 'Light mode' : 'Dark mode'" @mousedown.stop @click="toggleTheme()">
            <i class="fas" :class="theme === 'dark' ? 'fa-sun' : 'fa-moon'" />
          </button>
          <button class="wf-tool ww-tool ww-signout" title="Sign out of pathofexile.com" @mousedown.stop @click="signOut"><i class="fas fa-sign-out-alt" /></button>
          <button class="wf-tool wf-close ww-tool vs-close" title="Close (Esc)" @mousedown.stop @click="hideWindow"><i class="fas fa-times" /></button>
          </span>
        </div>
      </div>

      <template v-if="ready">
        <div v-if="error" class="ww-error"><i class="fas fa-exclamation-triangle" /> {{ error }}</div>

        <!-- wealth + breakdown -->
        <div class="ww-top">
          <div class="ww-card ww-wealth">
            <div class="ww-card-head">
              <span class="ww-card-title wf-heading">Wealth</span>
              <div class="ww-seg">
                <button :class="{ on: chartMode === 'total' }" @click="chartMode = 'total'">Total</button>
                <button :class="{ on: chartMode === 'hourly' }" @click="chartMode = 'hourly'">Hourly</button>
              </div>
              <select v-model="range" class="ww-select">
                <option value="0.0417">1 hour</option>
                <option value="0.25">6 hours</option>
                <option value="1">24 hours</option>
                <option value="7">7 days</option>
                <option value="30">30 days</option>
                <option value="0">All</option>
              </select>
              <span v-if="chart.change != null && chartMode === 'total'" class="ww-change" :class="{ neg: chart.change < 0 }"
                title="Change over the chosen time">
                {{ chart.change >= 0 ? '+' : '' }}{{ fmtDiv(chart.change) }}d
              </span>
              <span class="ww-net" :title="`${Math.round(totalChaos)} chaos`">
                <img :src="DIVINE_ICON" class="ww-cur" />{{ fmtDiv(totalDiv) }}
              </span>
            </div>
            <!-- hover anywhere: the nearest sync, its time and worth -->
            <div class="ww-chart-wrap" @mousemove="hoverChart" @mouseleave="hoverIdx = null">
              <span v-if="chart.points.length" class="ww-y-top">{{ fmtDiv(chart.hi) }}d</span>
              <svg class="ww-chart" viewBox="0 0 400 120" preserveAspectRatio="none">
                <defs>
                  <linearGradient id="ww-fill" x1="0" x2="0" y1="0" y2="1">
                    <stop offset="0" style="stop-color: var(--cyan); stop-opacity: 0.4" />
                    <stop offset="1" style="stop-color: var(--cyan); stop-opacity: 0" />
                  </linearGradient>
                </defs>
                <template v-if="chart.points.length">
                  <path :d="chart.area" fill="url(#ww-fill)" />
                  <path :d="chart.line" class="ww-line" fill="none" stroke-width="2" vector-effect="non-scaling-stroke" />
                  <line v-if="hovered" :x1="hovered.x" :x2="hovered.x" y1="0" y2="120" class="ww-guide" vector-effect="non-scaling-stroke" />
                </template>
              </svg>
              <!-- dots as plain circles (the graph stretches, they don't) -->
              <span v-for="(p, i) in chart.points" :key="i" class="ww-pt" :class="{ on: hoverIdx === i }"
                :style="{ left: `${p.x / 4}%`, top: `${p.y / 1.2}%` }" />
              <div v-if="hovered" class="ww-tip" :class="{ flip: hovered.x > 240 }"
                :style="{ left: `${hovered.x / 4}%`, top: `${Math.min(hovered.y / 1.2, 55)}%` }">
                <div class="ww-tip-date">{{ fmtLong(hovered.t) }}</div>
                <div class="ww-tip-val">
                  {{ chartMode === 'hourly' ? 'Income' : 'Wealth' }}:
                  <b>{{ fmtDiv(hovered.v) }}d</b><template v-if="chartMode === 'hourly'"> / hour</template>
                </div>
              </div>
            </div>
            <div class="ww-chart-axis">
              <template v-if="chart.points.length">
                <span v-for="(tick, i) in chart.ticks" :key="i">{{ tick }}</span>
              </template>
              <span v-else class="ww-dim ww-axis-empty">{{ selectedIds.length ? 'Sync to start the graph' : 'Tick tabs to see their graph' }}</span>
            </div>
          </div>

          <div class="ww-card ww-breakdown">
            <div class="ww-card-head">
              <span class="ww-card-title wf-heading">Breakdown</span>
              <div class="ww-seg">
                <button :class="{ on: breakdownMode === 'div' }" @click="breakdownMode = 'div'">Divines</button>
                <button :class="{ on: breakdownMode === 'pct' }" @click="breakdownMode = 'pct'">Percentage</button>
              </div>
            </div>
            <div class="ww-bd-grid">
              <div v-for="b in breakdown" :key="b.id" class="ww-bd" :class="{ viewing: viewTabId === b.id }"
                :title="`${b.name} · click to show only its items`" :style="{ '--tab': b.colour }" @click="openTab(b.id)">
                <img v-if="tabIcons.get(b.id)" :src="tabIcons.get(b.id)" class="ww-bd-icon" />
                <span v-else class="ww-bd-dot" :style="{ background: b.colour }" />
                <span class="ww-bd-name">{{ b.name }}</span>
                <span class="ww-bd-val">{{ breakdownMode === 'pct' ? `${b.pct.toFixed(1)}%` : fmtValue(b.chaos) }}</span>
              </div>
              <div v-if="!breakdown.length" class="ww-dim">Nothing priced yet.</div>
            </div>
          </div>
        </div>

        <div class="ww-body">
          <!-- left: tabs to sync / snapshots -->
          <div class="ww-side">
            <div class="ww-seg ww-seg-wide">
              <button :class="{ on: side === 'tabs' }" @click="side = 'tabs'">Tabs</button>
              <button :class="{ on: side === 'snapshots' }" @click="side = 'snapshots'">Snapshots</button>
            </div>

            <template v-if="side === 'tabs'">
              <div class="ww-tabs">
                <div v-if="!tabs.length" class="ww-dim ww-pad">
                  <span v-if="tabsBusy"><i class="fas fa-circle-notch fa-spin" /> Reading your tabs…</span>
                  <span v-else>No tabs read yet.</span>
                </div>
                <!-- each tab in its own stash colour, with the icon of what it holds -->
                <!-- the box: counted in the total or not · the rest: show its items -->
                <div v-for="t in tabs" :key="t.id" class="ww-tab" :class="{ on: selected.has(t.id), viewing: viewTabId === t.id }"
                  :style="{ '--tab': t.colour, '--tab-ink': tabInk(t.colour) }" :title="tabTitle(t)" @click="openTab(t.id)">
                  <i class="fas ww-tab-check" :class="selected.has(t.id) ? 'fa-check-square' : 'fa-square'"
                    :title="selected.has(t.id) ? 'Counted in the total: click to leave it out' : 'Click to count it in the total'"
                    @click.stop="toggleTab(t.id)" />
                  <img v-if="tabIcons.get(t.id)" :src="tabIcons.get(t.id)" class="ww-tab-icon" />
                  <i v-else class="fas ww-tab-icon ww-tab-icon-fa" :class="typeIcon(t.type)" />
                  <span class="ww-tab-name">{{ t.name }}</span>
                  <span v-if="selected.has(t.id) && tabValue(t.id) != null" class="ww-tab-val">{{ fmtValue(tabValue(t.id)!) }}</span>
                </div>
              </div>
              <div v-if="capNote" class="ww-cap-note"><i class="fas fa-shield-alt" /> {{ capNote }}</div>
              <div class="ww-side-foot">
                <button class="ww-link" :disabled="tabsBusy" @click="refreshTabs"><i class="fas fa-redo" /> Re-read tab list</button>
                <span class="ww-count" :class="{ full: selectedIds.length >= MAX_TABS }"
                  :title="`pathofexile.com allows 100 stash reads per 30 min; Vaal Street always stays under that`">
                  {{ selectedIds.length }} / {{ MAX_TABS }} tabs
                </span>
                <button v-if="tabs.length" class="ww-link" @click="selectAll(!selectedIds.length)">
                  {{ selectedIds.length ? 'None' : (tabs.length > MAX_TABS ? `First ${MAX_TABS}` : 'All') }}
                </button>
              </div>
            </template>

            <template v-else>
              <!-- the run: start → buy → run → done -->
              <div class="ww-run">
                <div class="ww-steps">
                  <span v-for="(s, i) in ['Start', 'Buy', 'Run', 'Done']" :key="s" class="ww-step"
                    :class="{ on: runStep === i, past: runStep > i }">
                    <b>{{ i + 1 }}</b>{{ s }}
                  </span>
                </div>

                <div v-if="runBusy" class="ww-phase ww-phase-busy"><i class="fas fa-circle-notch fa-spin" /> {{ runBusy }} <span class="ww-dim">{{ progressText }}</span></div>

                <template v-else-if="!activeRun">
                  <div class="ww-label">Investments are paid from</div>
                  <div v-if="payTab && !changingPay" class="ww-paytab" :style="{ '--tab': payTab.colour, '--tab-ink': tabInk(payTab.colour) }">
                    <img v-if="tabIcons.get(payTab.id)" :src="tabIcons.get(payTab.id)" />
                    <span>{{ payTab.name }}</span>
                    <button class="ww-link ww-change-pay" @click="changingPay = true">Change</button>
                  </div>
                  <select v-else class="ww-select ww-pay-select" :value="payTabId" @change="setPayTab(($event.target as HTMLSelectElement).value)">
                    <option value="" disabled>Pick the tab you buy with…</option>
                    <option v-for="t in tabs" :key="t.id" :value="t.id">{{ t.name }}</option>
                  </select>
                  <div class="ww-label">Loot is counted in</div>
                  <div class="ww-dim ww-small-text">
                    <template v-if="lootTabCount">
                      your {{ lootTabCount }} ticked tab{{ lootTabCount === 1 ? '' : 's' }} (Tabs list)<template v-if="payTab">{{ payTicked ? ', including ' : ', not ' }}{{ payTab.name }}</template>
                      <div v-if="payTab" class="ww-pay-note">
                        {{ payTicked
                          ? `${payTab.name} is ticked: what lands in it during the run counts too.`
                          : `${payTab.name} only pays: tick it to count the loot that lands in it.` }}
                      </div>
                    </template>
                    <span v-else class="ww-warn-text">Tick the tabs your loot goes into (Tabs list).</span>
                  </div>
                  <button class="wf-primary ww-btn ww-big-btn" :disabled="busy || !payTab || !lootTabCount" @click="startRun">
                    <i class="fas fa-camera" /> Start run
                  </button>
                </template>

                <template v-else-if="activeRun.phase === 'buying'">
                  <div class="ww-phase ww-phase-buy">
                    <i class="fas fa-shopping-cart" />
                    <div><b>Buying phase</b><br />Buy what you need for your strategy now, with {{ activeRun.payTabName }}.</div>
                  </div>
                  <div class="ww-alert">
                    <i class="fas fa-exclamation-triangle" />
                    <div><b>Change instance first</b> (hideout, a map, any zone), then press Done buying. pathofexile.com only saves your stash when you change zone.</div>
                  </div>
                  <button class="wf-primary ww-btn ww-big-btn" :disabled="busy" @click="doneBuying"><i class="fas fa-check" /> Done buying</button>
                  <button class="ww-link" @click="cancelRun">Cancel this run</button>
                </template>

                <template v-else>
                  <div class="ww-phase ww-phase-run">
                    <span class="ww-rec" />
                    <div class="ww-run-what"><b>Running your strategy</b><span class="ww-dim">Press Finish when you're done.</span></div>
                  </div>
                  <!-- the stopwatch -->
                  <div class="ww-watch" title="Time since Done buying">{{ fmtClock(runStats(activeRun).time) }}</div>
                  <div v-if="runStats(activeRun).profit != null" class="ww-snap-profit" :class="{ neg: runStats(activeRun).profit! < 0 }">
                    {{ runStats(activeRun).profit! >= 0 ? '+' : '' }}{{ fmtDiv(runStats(activeRun).profit! / runStats(activeRun).div) }}
                    <img :src="DIVINE_ICON" class="ww-cur" /><span class="ww-dim ww-small-text">so far</span>
                  </div>
                  <div class="ww-alert">
                    <i class="fas fa-exclamation-triangle" />
                    <div><b>Change instance first</b>, then press Finish, so every bit of your loot is counted.</div>
                  </div>
                  <div class="ww-pair">
                    <button class="wf-ghost ww-btn" :disabled="busy" title="See how it's going (the run goes on)" @click="snapshotNow"><i class="fas fa-camera" /> Check</button>
                    <button class="wf-primary ww-btn" :disabled="busy" @click="finishRun"><i class="fas fa-flag-checkered" /> Finish</button>
                  </div>
                  <div class="ww-dim ww-small-text">Checks itself every {{ autoSyncEvery }} min.</div>
                </template>
              </div>

              <!-- every run of this league -->
              <div class="ww-snaps">
                <div v-for="r in leagueRuns" :key="r.id" class="ww-snap" :class="{ on: shownRun?.id === r.id }" @click="shownRunId = r.id">
                  <div class="ww-snap-when">
                    {{ fmtDate(r.a.t) }} · {{ fmtDuration(runStats(r).time) }}
                    <span v-if="r.phase !== 'done'" class="ww-live">{{ r.phase === 'buying' ? 'buying' : 'running' }}</span>
                  </div>
                  <div v-if="runStats(r).profit != null" class="ww-snap-val" :class="{ neg: runStats(r).profit! < 0 }">
                    {{ runStats(r).profit! >= 0 ? '+' : '' }}{{ fmtDiv(runStats(r).profit! / runStats(r).div) }} div
                    <span v-if="runStats(r).perHour != null" class="ww-dim">· {{ fmtDiv(runStats(r).perHour! / runStats(r).div) }}/h</span>
                  </div>
                  <div v-else class="ww-dim ww-small-text">no result yet</div>
                  <button class="ww-x" title="Delete this run" @click.stop="deleteRun(r.id)"><i class="fas fa-trash" /></button>
                </div>
              </div>
            </template>
          </div>

          <!-- right (Snapshots): the run's report -->
          <div v-if="side === 'snapshots'" class="ww-main">
            <template v-if="shownRun">
              <div class="ww-report-head">
                <span class="wf-heading">Run · {{ fmtLong(shownRun.a.t) }}</span>
                <span class="ww-dim">paid from {{ shownRun.payTabName }}{{ shownRun.payCounts ? ' (its loot counted too)' : '' }}<template v-if="shownRun.phase !== 'done'"> · {{ shownRun.phase === 'buying' ? 'buying now' : 'running now' }}</template></span>
              </div>
              <div class="ww-cards">
                <div class="ww-stat">
                  <span>Started with</span>
                  <b>{{ fmtValue(runStats(shownRun).start) }}</b>
                </div>
                <div class="ww-stat ww-stat-cost">
                  <span>Investment</span>
                  <b>{{ shownRun.b ? `−${fmtValue(runStats(shownRun).cost)}` : '…' }}</b>
                </div>
                <div class="ww-stat ww-stat-rev">
                  <span>Revenue</span>
                  <b>{{ runStats(shownRun).revenue != null ? `+${fmtValue(runStats(shownRun).revenue!)}` : '…' }}</b>
                </div>
                <div class="ww-stat ww-stat-profit" :class="{ neg: (runStats(shownRun).profit ?? 0) < 0, none: runStats(shownRun).profit == null }">
                  <span>Profit</span>
                  <b>
                    <template v-if="runStats(shownRun).profit != null">
                      {{ runStats(shownRun).profit! >= 0 ? '+' : '' }}{{ fmtDiv(runStats(shownRun).profit! / runStats(shownRun).div) }}
                      <img :src="DIVINE_ICON" class="ww-cur" />
                    </template>
                    <template v-else>…</template>
                  </b>
                </div>
                <div class="ww-stat">
                  <span>Time</span>
                  <b>{{ fmtDuration(runStats(shownRun).time) }}</b>
                </div>
                <div class="ww-stat" :class="{ neg: (runStats(shownRun).perHour ?? 0) < 0 }">
                  <span>Per hour</span>
                  <b :title="runStats(shownRun).perHour == null ? 'Shown once the run lasts 5 minutes' : ''">{{ runStats(shownRun).perHour != null ? `${fmtDiv(runStats(shownRun).perHour! / runStats(shownRun).div)} div` : '…' }}</b>
                </div>
              </div>
              <div class="ww-tools">
                <div class="ww-search"><i class="fas fa-search" /><input v-model="search" placeholder="Search" /></div>
                <span class="ww-dim">{{ runRows.length }} items changed · prices at each snapshot</span>
              </div>
              <div class="ww-table">
                <div class="ww-tr ww-th ww-run-tr">
                  <span class="c-name">Item</span>
                  <span class="c-num" :title="`Out of ${shownRun.payTabName} while buying`">Spent</span>
                  <span class="c-num" title="Into your ticked tabs, start → finish">Looted</span>
                  <span class="c-num">Price</span>
                  <span class="c-num">Value</span>
                </div>
                <div class="ww-rows">
                  <div v-for="x in runRows" :key="x.key" class="ww-tr ww-run-tr">
                    <span class="c-name"><img v-if="x.icon" :src="x.icon" class="ww-icon" /><span v-else class="ww-icon ww-icon-empty" /><span class="ww-name">{{ x.name }}</span></span>
                    <span class="c-num" :class="x.spent ? 'down' : 'ww-dim'">{{ x.spent ? '−' + x.spent.toLocaleString() : '–' }}</span>
                    <span class="c-num" :class="x.looted > 0 ? 'up' : x.looted < 0 ? 'down' : 'ww-dim'">{{ x.looted ? (x.looted > 0 ? '+' : '') + x.looted.toLocaleString() : '–' }}</span>
                    <span class="c-num">{{ fmtValue(x.each) }}</span>
                    <span class="c-num ww-total" :class="{ down: x.value < 0 }">{{ x.value >= 0 ? '+' : '' }}{{ fmtValue(x.value) }}</span>
                  </div>
                  <div v-if="!runRows.length" class="ww-empty">
                    <template v-if="shownRun.phase === 'buying'">Buy your investments, then press "Done buying".</template>
                    <template v-else-if="shownRun.phase === 'running' && !shownRun.c">Run your strategy, then press "Finish" (or "Check" to peek).</template>
                    <template v-else>Nothing changed in these tabs.</template>
                  </div>
                </div>
              </div>
            </template>
            <div v-else class="ww-empty ww-how">
              <div class="wf-heading">How a run works</div>
              <ol>
                <li><b>Start run</b> — a snapshot of your paying tab and your ticked (loot) tabs.</li>
                <li><b>Buying phase</b> — buy your maps and investments, then press <b>Done buying</b>.</li>
                <li><b>Run</b> your strategy, then press <b>Finish</b>.</li>
                <li><b>Investment</b> = what left the paying tab · <b>Revenue</b> = what came into your ticked tabs · <b>Profit</b> = revenue − investment (green) or a loss (red), per hour.</li>
              </ol>
            </div>
          </div>

          <!-- right (Tabs): the items -->
          <div v-else class="ww-main">
            <div class="ww-tools">
              <div class="ww-search"><i class="fas fa-search" /><input v-model="search" placeholder="Search" /></div>
              <template v-if="viewTab">
                <button class="ww-viewing" :style="{ '--tab': viewTab.colour, '--tab-ink': tabInk(viewTab.colour) }"
                  title="Show every ticked tab again" @click="viewTabId = null">
                  <img v-if="tabIcons.get(viewTab.id)" :src="tabIcons.get(viewTab.id)" />
                  Showing: {{ viewTab.name }} <i class="fas fa-times" />
                </button>
                <span class="ww-dim">{{ rows.length }} items<template v-if="!selected.has(viewTab.id)"> · not counted in the total</template></span>
              </template>
              <span v-else class="ww-dim">{{ rows.length }} items · {{ selectedIds.length }} tabs</span>
            </div>
            <div class="ww-table">
              <div class="ww-tr ww-th">
                <span class="c-name" @click="sortBy('name')">Name <i :class="sortIcon('name')" /></span>
                <span class="c-tab" @click="sortBy('tab')">Tab <i :class="sortIcon('tab')" /></span>
                <span class="c-num" @click="sortBy('qty')">Quantity <i :class="sortIcon('qty')" /></span>
                <span class="c-num" @click="sortBy('each')">Price <i :class="sortIcon('each')" /></span>
                <span class="c-num" @click="sortBy('total')">Total <i :class="sortIcon('total')" /></span>
              </div>
              <div class="ww-rows">
                <div v-for="r in shownRows" :key="r.id" class="ww-tr">
                  <span class="c-name"><img v-if="r.icon" :src="r.icon" class="ww-icon" /><span v-else class="ww-icon ww-icon-empty" /><span class="ww-name">{{ r.name }}</span></span>
                  <span class="c-tab">
                    <template v-if="r.diff != null"><span :class="r.diff >= 0 ? 'up' : 'down'">{{ r.diff >= 0 ? '+' : '' }}{{ r.diff }}</span></template>
                    <template v-else>
                      <img v-if="tabIcons.get(r.tabId)" :src="tabIcons.get(r.tabId)" class="ww-cell-icon" />
                      <span v-else class="ww-bd-dot" :style="{ background: r.tabColour }" />
                      <span class="ww-cell-tab" :style="{ '--tab': r.tabColour }">{{ r.tabName }}</span>
                    </template>
                  </span>
                  <span class="c-num">{{ r.qty.toLocaleString() }}</span>
                  <span class="c-num">{{ fmtValue(r.each) }}</span>
                  <span class="c-num ww-total" :class="{ down: r.total < 0 }">{{ fmtValue(r.total) }}</span>
                </div>
                <div v-if="!shownRows.length" class="ww-empty">
                  <template v-if="side === 'tabs' && viewTab && !wealth.holdings[league]?.[viewTab.id]">
                    {{ viewTab.name }} hasn't been read yet: tick its box, then press Sync.
                  </template>
                  <template v-else-if="side === 'tabs' && viewTab">Nothing priced in {{ viewTab.name }}.</template>
                  <template v-else-if="!selectedIds.length">Tick the tabs to count on the left (the box), then press Sync.</template>
                  <template v-else-if="!hasHoldings">Press Sync to read the chosen tabs.</template>
                  <template v-else>Nothing matches.</template>
                </div>
                <div v-if="sortedRows.length > shownRows.length" class="ww-dim ww-pad">… and {{ sortedRows.length - shownRows.length }} cheaper lines</div>
              </div>
            </div>
          </div>
        </div>
      </template>
      <!-- drag to make the panel bigger or smaller; double-click: back to normal -->
      <div v-if="ready" class="ww-grip" title="Drag to resize · double-click for the normal size" @mousedown.stop.prevent="resizeWindow" @dblclick.stop="resetSize" />
    </div>

    <!-- a new version on GitHub: a pixel notice drops in -->
    <transition name="vs-drop">
      <div v-if="update && (!updateHidden || updating)" class="vs-upd wf-frame">
        <div class="vs-upd-art" :class="{ busy: updating }">
          <span class="vs-px-arrow" />
          <span class="vs-px-tray" />
        </div>
        <div class="vs-upd-body">
          <div class="vs-upd-title wf-wordmark">{{ updateStage === 'install' ? 'INSTALLING' : updating ? 'DOWNLOADING' : 'UPDATE READY' }}</div>
          <div class="vs-upd-ver">
            <span class="vs-upd-old">v{{ update.current }}</span>
            <span class="vs-upd-to">▶</span>
            <b>v{{ update.version }}</b>
          </div>
          <div v-if="update.notes && !updating" class="vs-upd-notes">{{ update.notes }}</div>
          <template v-if="updating">
            <!-- 20 blocks, filled one by one -->
            <div class="vs-upd-bar"><span v-for="i in 20" :key="i" :class="{ on: i <= updateCells }" /></div>
            <div class="vs-upd-pct">{{ updateStage === 'install' ? 'Vaal Street restarts in a moment…' : `${updatePct}%` }}</div>
          </template>
          <div v-else class="vs-upd-btns">
            <button class="wf-primary vs-upd-go" @click="installUpdate"><i class="fas fa-download" /> Install &amp; restart</button>
            <button class="wf-ghost vs-upd-later" @click="updateHidden = true">Later</button>
          </div>
        </div>
      </div>
    </transition>
  </div>
</template>

<script setup lang="ts">
// Vaal Street: your stash's worth, its graph over time and each run's profit.
// The window sits over the game: Ctrl+W, Esc or a click elsewhere puts it away.
import { computed, nextTick, onBeforeUnmount, shallowRef, watch } from 'vue'
import { onEvent } from './ipc'
import { signedIn, signIn as poeSignIn, recheckSession } from './session'
import { theme, toggleTheme } from './theme'
import { usePoeninja } from './prices'
import { useLeagues } from './leagues'
import { knownIcon, DIVINE_ICON } from './icons'
import {
  wealth, loadWealth, saveWealth, invoke, holdingsOf, mapIcons, loadMapIcons,
  type StashTab, type SlimItem, type Priceable, type Run, type RunSnap
} from './wealth'

const leagues = useLeagues()
const ninja = usePoeninja()
// the league list
void leagues.load()
const rootEl = shallowRef<HTMLElement | null>(null)

const league = computed(() => leagues.selectedId.value ?? '')
/** the league picker in the title bar (saved in the settings) */
const leagueChoice = computed({
  get: () => leagues.selectedId.value ?? '',
  set: (id: string) => { leagues.selectedId.value = id }
})
const account = shallowRef('')
const error = shallowRef('')
/** signed in through the sign-in window, account known: the panel itself */
const ready = computed(() => signedIn.value && Boolean(account.value))

// the saved ticked tabs, holdings, history and runs: read at start-up, so the
// panel shows the last wealth at once (and nothing saves over an unread file)
void loadWealth()

onEvent('poe-account', (e) => {
  account.value = e.account
  error.value = ''
})


/** asking whether the saved sign-in works (when the window opens) */
const checking = shallowRef(false)
/** the sign-in window is open: the panel comes back by itself once signed in */
const waitingSignIn = shallowRef(false)
let signInPoll: ReturnType<typeof setInterval> | undefined

/** opens pathofexile.com's sign-in page in Vaal Street's own browser window;
 *  once signed in there, it reads the account name and closes by itself */
function signIn () {
  poeSignIn()
  waitingSignIn.value = true
  clearInterval(signInPoll)
  const started = Date.now()
  signInPoll = setInterval(() => {
    if (ready.value || Date.now() - started > 15 * 60_000) {
      clearInterval(signInPoll)
      waitingSignIn.value = false
      return
    }
    void recheckSession().then(readAccount)
  }, 2500)
}
onBeforeUnmount(() => clearInterval(signInPoll))

async function signOut () {
  await invoke('wealth_sign_out').catch(() => {})
  account.value = ''
  error.value = ''
  void recheckSession()
}

async function readAccount () {
  if (!signedIn.value) { account.value = ''; return }
  const a = await invoke<{ signedIn: boolean, account?: string }>('wealth_account').catch(() => null)
  account.value = a?.account ?? ''
}

// signed in (or out): the panel or the sign-in screen
watch(ready, (now) => {
  if (now) {
    // signed in from this panel: bring it back with the graph and the tabs
    if (waitingSignIn.value) {
      waitingSignIn.value = false
      clearInterval(signInPoll)
    }
    if (league.value && !(wealth.tabs[league.value]?.length)) void refreshTabs()
  }
})
watch(signedIn, (now) => { if (now) void readAccount(); else account.value = '' })

// ------------------------------------------------ tabs
const tabs = computed<StashTab[]>(() => wealth.tabs[league.value] ?? [])
const selected = computed(() => new Set(wealth.selected[league.value] ?? []))
const selectedIds = computed(() => tabs.value.filter(t => selected.value.has(t.id)).map(t => t.id))
const tabsBusy = shallowRef(false)

async function refreshTabs () {
  if (!league.value) return
  tabsBusy.value = true
  error.value = ''
  try {
    // the saved file first: never write a tab list over an unread file
    await loadWealth()
    const list = await invoke<StashTab[]>('wealth_tabs', { league: league.value })
    wealth.tabs[league.value] = list
    saveWealth()
  } catch (e) {
    error.value = `Could not read the tab list: ${String(e)}`
  } finally {
    tabsBusy.value = false
  }
}

/**
 * pathofexile.com lets an account read 100 stash tabs per 30 minutes (and 30
 * per minute). Vaal Street always stays a few requests under it: at most 50 tabs ticked, so
 * one full sync always fits, with room left for the website or other tools.
 */
const MAX_TABS = 50
const capNote = shallowRef('')
let capTimer: ReturnType<typeof setTimeout> | undefined
function showCap (text: string) {
  capNote.value = text
  clearTimeout(capTimer)
  capTimer = setTimeout(() => { capNote.value = '' }, 6000)
}

function toggleTab (id: string) {
  const set = new Set(selectedIds.value)
  if (set.has(id)) {
    set.delete(id)
    // the tab left the total: stop showing only it
    if (viewTabId.value === id) viewTabId.value = null
  } else if (set.size >= MAX_TABS) {
    showCap(`At most ${MAX_TABS} tabs, so a sync stays under pathofexile.com's limit. Untick one first.`)
    return
  } else set.add(id)
  wealth.selected[league.value] = [...set]
  saveWealth()
}
function selectAll (on: boolean) {
  wealth.selected[league.value] = on ? tabs.value.slice(0, MAX_TABS).map(t => t.id) : []
  if (!on) viewTabId.value = null
  if (on && tabs.value.length > MAX_TABS) showCap(`Picked the first ${MAX_TABS} tabs (the most one sync may read).`)
  saveWealth()
}

/** auto-sync during a session: never more than 50 tab reads per 30 minutes */
const autoSyncEvery = computed(() => Math.max(wealth.autoSyncMin, Math.ceil(selectedIds.value.length * 30 / MAX_TABS)))
function tabTitle (t: StashTab) {
  const h = wealth.holdings[league.value]?.[t.id]
  return h ? `${t.name} · read ${fmtAgo(h.at)} ago` : `${t.name} · not read yet`
}

// ------------------------------------------------ prices
const rate = computed(() => ninja.xchgRate.value ?? 200)

const priceCache = new Map<string, number>()
watch(() => ninja.xchgRate.value, () => priceCache.clear())

function priceOf (p: Pick<Priceable, 'key' | 'ask'>): number {
  void ninja.xchgRate.value // re-price when poe.ninja loaded
  const hit = priceCache.get(p.key)
  if (hit != null) return hit
  let chaos = 0
  if (p.ask[0]?.name === 'Chaos Orb' && p.ask[0].ns === 'ITEM') chaos = 1
  else {
    for (const a of p.ask) {
      const found = ninja.findPriceByQuery({ ns: a.ns, name: a.name, variant: a.variant })
      if (found) { chaos = found.chaos; break }
    }
  }
  if (ninja.xchgRate.value) priceCache.set(p.key, chaos)
  return chaos
}

interface Row { id: string, key: string, name: string, icon: string, tabId: string, tabName: string, tabColour: string, qty: number, each: number, total: number, diff?: number }

/** an item's picture: the stash's own, else the price checker's item database */
const iconCache = new Map<string, string>()
function iconFor (p: { key: string, icon: string, ask: Priceable['ask'] }): string {
  if (p.icon) return p.icon
  // maps from the Map tab: poe.ninja's picture ("Map (Tier 11)", "Nightmare Map")
  for (const a of p.ask) {
    const map = mapIcons.value[a.name]
    if (map) return map
  }
  const hit = iconCache.get(p.key)
  if (hit != null) return hit
  let found = ''
  for (const a of p.ask) {
    found = knownIcon(a.name)
    if (found) break
  }
  iconCache.set(p.key, found)
  return found
}

/**
 * Before a tab was read: an item that stands for it, by its kind or its name
 * ("- Fossils -" → a fossil), pictured from the price checker's item list.
 */
const KIND_ITEM: Array<[RegExp, 'ITEM' | 'DIVINATION_CARD' | 'GEM' | 'UNIQUE', string]> = [
  [/fossil|resonator|delve/i, 'ITEM', 'Dense Fossil'],
  [/\boils?\b|blight|anoint/i, 'ITEM', 'Golden Oil'],
  [/essence/i, 'ITEM', 'Deafening Essence of Greed'],
  [/catalyst|ultimatum/i, 'ITEM', 'Prismatic Catalyst'],
  [/delirium|simulacrum/i, 'ITEM', 'Simulacrum Splinter'],
  [/card|divination/i, 'DIVINATION_CARD', 'Divination Card'],
  [/gem/i, 'GEM', 'Empower Support'],
  [/\bmaps?\b/i, 'ITEM', 'Blighted Map'],
  [/fragment|scarab|splinter/i, 'ITEM', 'Fragment of the Phoenix'],
  [/unique/i, 'UNIQUE', 'Headhunter'],
  [/flask/i, 'ITEM', 'Divine Life Flask'],
  [/jewel/i, 'ITEM', 'Lethal Pride'],
  [/heist|contract|blueprint|marker/i, 'ITEM', 'Rogue\'s Marker'],
  [/expedition|logbook/i, 'ITEM', 'Expedition Logbook'],
  [/tainted|ritual/i, 'ITEM', 'Tainted Chaos Orb'],
  [/mirror/i, 'ITEM', 'Mirror of Kalandra'],
  [/divine/i, 'ITEM', 'Divine Orb'],
  [/currency|sale|trade|\bc\b|orbs?/i, 'ITEM', 'Chaos Orb']
]
const TYPE_ITEM: Record<string, string> = {
  CurrencyStash: 'currency',
  MapStash: 'maps',
  FragmentStash: 'fragment',
  DivinationCardStash: 'divination',
  EssenceStash: 'essence',
  DelveStash: 'fossil',
  BlightStash: 'oil',
  DeliriumStash: 'delirium',
  UltimatumStash: 'ultimatum',
  UniqueStash: 'unique',
  GemStash: 'gem',
  FlaskStash: 'flask'
}
function kindIcon (t: StashTab): string {
  // the tab's kind first (a Currency tab named "Loot" is still currency), then its name
  const text = `${TYPE_ITEM[t.type] ?? ''} ${t.name}`
  for (const [re, ns, name] of KIND_ITEM) {
    if (re.test(TYPE_ITEM[t.type] ?? '') || (!TYPE_ITEM[t.type] && re.test(text))) {
      return knownIcon(name)
    }
  }
  return ''
}

/** tabs that always wear the same picture (the player's choice), by name */
const FIXED_ICON: Array<[RegExp, string]> = [
  // the Breach portal (PoE Wiki: Breach_Portal_Effect_inventory_icon.png), shipped in images\
  [/breach/i, '/images/breach-portal.png']
]

/**
 * A tab's icon: a fixed one (Breach), else its kind's / name's. It stays the
 * same after a sync (the player likes these better than the tab's top item).
 */
const tabIcons = computed(() => {
  const out = new Map<string, string>()
  for (const t of tabs.value) {
    const fixed = FIXED_ICON.find(([re]) => re.test(t.name))
    const icon = fixed ? fixed[1] : kindIcon(t)
    if (icon) out.set(t.id, icon)
  }
  return out
})

/** before a tab was read: an icon for its kind */
const TYPE_ICON: Record<string, string> = {
  CurrencyStash: 'fa-coins',
  MapStash: 'fa-map',
  FragmentStash: 'fa-puzzle-piece',
  DivinationCardStash: 'fa-clone',
  EssenceStash: 'fa-tint',
  DelveStash: 'fa-mountain',
  BlightStash: 'fa-flask',
  DeliriumStash: 'fa-eye',
  MetamorphStash: 'fa-dna',
  UltimatumStash: 'fa-skull',
  UniqueStash: 'fa-star',
  GemStash: 'fa-gem',
  FlaskStash: 'fa-wine-bottle',
  QuadStash: 'fa-th-large'
}
function typeIcon (type: string) {
  return TYPE_ICON[type] ?? 'fa-box'
}

/** dark or light letters, whichever reads on the tab's own colour */
function tabInk (hex: string) {
  const m = /^#?([0-9a-f]{2})([0-9a-f]{2})([0-9a-f]{2})$/i.exec(hex)
  if (!m) return '#f2f2f2'
  const [r, g, b] = [m[1], m[2], m[3]].map(x => parseInt(x, 16) / 255)
  const lum = 0.2126 * r + 0.7152 * g + 0.0722 * b
  return lum > 0.5 ? '#16130d' : '#f6f2e8'
}

function rowsOfTab (t: StashTab): Row[] {
  const out: Row[] = []
  for (const e of wealth.holdings[league.value]?.[t.id]?.entries ?? []) {
    const each = priceOf(e)
    if (each <= 0) continue
    out.push({ id: `${t.id}|${e.key}`, key: e.key, name: e.name, icon: iconFor(e), tabId: t.id, tabName: t.name, tabColour: t.colour, qty: e.qty, each, total: each * e.qty })
  }
  return out
}

/** every ticked tab: what the total and the breakdown count */
const holdingRows = computed<Row[]>(() => tabs.value.filter(t => selected.value.has(t.id)).flatMap(rowsOfTab))

/** clicking a tab (its name) or its breakdown entry: only that tab's items */
const viewTabId = shallowRef<string | null>(null)
const viewTab = computed(() => tabs.value.find(t => t.id === viewTabId.value) ?? null)
function openTab (id: string) {
  viewTabId.value = viewTabId.value === id ? null : id
  side.value = 'tabs'
}
watch(league, () => { viewTabId.value = null })
const listRows = computed<Row[]>(() => viewTab.value ? rowsOfTab(viewTab.value) : holdingRows.value)
const hasHoldings = computed(() => selectedIds.value.some(id => wealth.holdings[league.value]?.[id]))

const totalChaos = computed(() => holdingRows.value.reduce((s, r) => s + r.total, 0))
const totalDiv = computed(() => totalChaos.value / rate.value)

function tabValue (id: string): number | null {
  const h = wealth.holdings[league.value]?.[id]
  if (!h) return null
  return h.entries.reduce((s, e) => s + priceOf(e) * e.qty, 0)
}

const breakdownMode = shallowRef<'div' | 'pct'>('div')
const breakdown = computed(() => {
  const total = totalChaos.value || 1
  return tabs.value
    .filter(t => selected.value.has(t.id))
    .map(t => ({ id: t.id, name: t.name, colour: t.colour, chaos: tabValue(t.id) ?? 0 }))
    .filter(b => b.chaos > 0)
    .sort((a, b) => b.chaos - a.chaos)
    .slice(0, 12)
    .map(b => ({ ...b, pct: b.chaos / total * 100 }))
})

// ------------------------------------------------ sync
const busy = shallowRef(false)
const progressText = shallowRef('')
let waitTimer: ReturnType<typeof setInterval> | undefined

/** reads the ticked tabs, plus `extra` ones (a run's paying tab), at most 50 */
async function sync (extra: string[] = []): Promise<boolean> {
  if (busy.value || !league.value) return false
  await loadWealth()
  // never more than 50 tabs in one sync (a list saved before the cap may hold more)
  const want = new Set([...selectedIds.value, ...extra])
  const list = tabs.value.filter(t => want.has(t.id)).slice(0, MAX_TABS)
  if (!list.length) return false
  busy.value = true
  error.value = ''
  let ok = true
  try {
    for (let i = 0; i < list.length; i++) {
      const t = list[i]
      // the tab's name without its "- ... -" decoration, so the button stays short
      const short = t.name.replace(/^[\s\-~#=*_.]+|[\s\-~#=*_.]+$/g, '') || t.name
      const label = `${short} ${i + 1}/${list.length}`
      progressText.value = label
      clearInterval(waitTimer)
      // the wait pathofexile.com's limits ask for, counted down
      waitTimer = setInterval(() => {
        void invoke<number>('wealth_wait').then(s => {
          progressText.value = s >= 1 ? `${label} · ${s >= 60 ? `waiting ${fmtWait(s)}` : `next in ${Math.ceil(s)} s`}` : label
        }).catch(() => {})
      }, 1000)
      const items = await invoke<SlimItem[]>('wealth_tab', { league: league.value, index: t.index, id: t.id, kind: t.type })
      ;(wealth.holdings[league.value] ??= {})[t.id] = holdingsOf(items)
    }
  } catch (e) {
    ok = false
    error.value = `Sync stopped: ${String(e)}`
  } finally {
    clearInterval(waitTimer)
    busy.value = false
  }
  if (ok) {
    recordPoint()
    wealth.lastSync[league.value] = Date.now()
  }
  saveWealth()
  return ok
}

function recordPoint () {
  if (!ninja.xchgRate.value) return // no prices: a point of 0 would ruin the graph
  const perTab: Record<string, number> = {}
  for (const id of selectedIds.value) perTab[id] = Math.round((tabValue(id) ?? 0) * 100) / 100
  const point = { t: Date.now(), chaos: totalChaos.value, div: totalDiv.value, tabs: perTab }
  const list = (wealth.history[league.value] ??= [])
  list.push(point)
  if (list.length > 3000) list.splice(0, list.length - 3000)
}

/** when the last sync finished (remembered in wealth.json) */
const lastSyncAt = computed(() => {
  const saved = wealth.lastSync?.[league.value]
  if (saved) return saved
  // files from before lastSync: the newest tab read
  const held = Object.values(wealth.holdings[league.value] ?? {})
  return held.length ? Math.max(...held.map(h => h.at)) : 0
})
const syncedAgo = computed(() => {
  void clock.value
  return lastSyncAt.value ? fmtAgo(lastSyncAt.value) : ''
})

// ------------------------------------------------ runs (Snapshots tab)
// 1) Start: snapshot of the paying tab + the loot (ticked) tabs
// 2) Buying phase: the player buys maps / scarabs / investments from the
//    paying tab, then presses "Done buying": snapshot → the investment
// 3) Running: they run their strategy, then press "Finish": snapshot →
//    revenue and profit (green or red), per hour

const side = shallowRef<'tabs' | 'snapshots'>('tabs')

/** the paying tab of this league (remembered) */
const payTabId = computed(() => wealth.payTab[league.value] ?? '')
const payTab = computed(() => tabs.value.find(t => t.id === payTabId.value) ?? null)
const changingPay = shallowRef(false)
/** the ticked tabs that count as loot (never the paying tab) */
const lootTabCount = computed(() => selectedIds.value.length)
/** the paying tab is ticked too: its loot counts */
const payTicked = computed(() => Boolean(payTabId.value) && selectedIds.value.includes(payTabId.value))
function setPayTab (id: string) {
  if (!id) return
  wealth.payTab[league.value] = id
  changingPay.value = false
  saveWealth()
}

const leagueRuns = computed(() => wealth.runs.filter(r => r.league === league.value).slice().sort((a, b) => b.a.t - a.a.t))
const activeRun = computed(() => wealth.runs.find(r => r.league === league.value && r.phase !== 'done') ?? null)
const shownRunId = shallowRef<string | null>(null)
/** the step shown lit: 0 start, 1 buying, 2 running */
const runStep = computed(() => !activeRun.value ? 0 : activeRun.value.phase === 'buying' ? 1 : 2)
const shownRun = computed(() => leagueRuns.value.find(r => r.id === shownRunId.value) ?? activeRun.value ?? leagueRuns.value[0] ?? null)

function runTabIds (r: { payTabId: string, lootTabIds: string[] }) {
  return [...new Set([r.payTabId, ...r.lootTabIds])]
}

/** what the run's tabs hold now, and today's prices */
function takeSnap (run: Run): RunSnap {
  const held = wealth.holdings[league.value] ?? {}
  const snap: RunSnap = { t: Date.now(), rate: rate.value, qty: {}, payQty: {}, lootQty: {}, price: {} }
  for (const id of runTabIds(run)) {
    for (const e of held[id]?.entries ?? []) {
      snap.qty[e.key] = (snap.qty[e.key] ?? 0) + e.qty
      if (id === run.payTabId) snap.payQty[e.key] = (snap.payQty[e.key] ?? 0) + e.qty
      else snap.lootQty![e.key] = (snap.lootQty![e.key] ?? 0) + e.qty
      if (snap.price[e.key] == null) snap.price[e.key] = priceOf(e)
      run.names[e.key] ??= { name: e.name, icon: e.icon, ask: e.ask }
    }
  }
  return snap
}

/** the loot tabs' counts (runs saved before lootQty: all tabs minus the paying tab) */
function lootOf (s: RunSnap): Record<string, number> {
  if (s.lootQty) return s.lootQty
  const out: Record<string, number> = {}
  for (const key in s.qty) {
    const n = s.qty[key] - (s.payQty[key] ?? 0)
    if (n) out[key] = n
  }
  return out
}

function worth (s: RunSnap | undefined, part: 'all' | 'pay' | 'loot' = 'all') {
  if (!s) return 0
  const qty = part === 'pay' ? s.payQty : part === 'loot' ? lootOf(s) : s.qty
  let sum = 0
  for (const key in qty) sum += qty[key] * (s.price[key] ?? 0)
  return sum
}

/** the run's numbers (chaos) */
function runStats (r: Run) {
  const start = worth(r.a)
  // the investment: what left the paying tab while buying
  const cost = r.b ? Math.max(0, worth(r.a, 'pay') - worth(r.b, 'pay')) : 0
  // the revenue: what came into the other ticked tabs, start → finish, plus
  // the paying tab's own loot (Done buying → finish) only if it was ticked
  const payLoot = r.payCounts && r.b && r.c ? worth(r.c, 'pay') - worth(r.b, 'pay') : 0
  const revenue = r.c ? worth(r.c, 'loot') - worth(r.a, 'loot') + payLoot : null
  const profit = revenue != null ? revenue - cost : null
  const end = r.c ? worth(r.c) : null
  const runStart = r.b?.t ?? r.a.t
  const runEnd = r.c?.t ?? (r.phase === 'done' ? runStart : clock.value)
  const time = Math.max(0, (r.phase === 'running' ? clock.value : runEnd) - runStart)
  const div = r.c?.rate ?? r.b?.rate ?? r.a.rate
  // per hour only means something after a few minutes
  const perHour = profit != null && time >= 5 * 60_000 ? profit / (time / 3_600_000) : null
  return { start, cost, end, profit, revenue, time, perHour, div }
}

const runBusy = shallowRef('')

async function startRun () {
  if (!payTabId.value || activeRun.value) return
  const lootTabIds = selectedIds.value.filter(id => id !== payTabId.value)
  if (lootTabIds.length + 1 > MAX_TABS) { error.value = `At most ${MAX_TABS} tabs in a run.`; return }
  runBusy.value = 'Taking the starting snapshot…'
  const ok = await sync([payTabId.value])
  runBusy.value = ''
  if (!ok) return
  const run: Run = {
    id: `${Date.now()}`,
    league: league.value,
    payTabId: payTabId.value,
    payTabName: payTab.value?.name ?? '',
    lootTabIds,
    payCounts: selectedIds.value.includes(payTabId.value),
    phase: 'buying',
    a: { t: 0, rate: 0, qty: {}, payQty: {}, price: {} },
    names: {}
  }
  run.a = takeSnap(run)
  wealth.runs.push(run)
  shownRunId.value = run.id
  saveWealth()
}

async function doneBuying () {
  const run = activeRun.value
  if (!run || run.phase !== 'buying') return
  runBusy.value = 'Counting what you bought…'
  const ok = await sync([run.payTabId, ...run.lootTabIds])
  runBusy.value = ''
  if (!ok) return
  run.b = takeSnap(run)
  run.phase = 'running'
  saveWealth()
}

/** during the run: a look at how it goes (the run goes on) */
async function snapshotNow () {
  const run = activeRun.value
  if (!run || run.phase !== 'running') return
  runBusy.value = 'Snapshot…'
  const ok = await sync([run.payTabId, ...run.lootTabIds])
  runBusy.value = ''
  if (!ok) return
  run.c = takeSnap(run)
  saveWealth()
}

async function finishRun () {
  const run = activeRun.value
  if (!run || run.phase !== 'running') return
  runBusy.value = 'Counting your loot…'
  const ok = await sync([run.payTabId, ...run.lootTabIds])
  runBusy.value = ''
  if (!ok) return
  run.c = takeSnap(run)
  run.phase = 'done'
  shownRunId.value = run.id
  saveWealth()
}

function cancelRun () {
  const run = activeRun.value
  if (!run) return
  wealth.runs.splice(wealth.runs.indexOf(run), 1)
  saveWealth()
}

function deleteRun (id: string) {
  const i = wealth.runs.findIndex(r => r.id === id)
  if (i >= 0) wealth.runs.splice(i, 1)
  if (shownRunId.value === id) shownRunId.value = null
  saveWealth()
}

/** the report's table: every item that changed, and when */
/** spent: out of the paying tab while buying · looted: into the ticked tabs, start → finish */
interface RunRow { key: string, name: string, icon: string, spent: number, looted: number, each: number, value: number }
const runRows = computed<RunRow[]>(() => {
  const r = shownRun.value
  if (!r) return []
  const loot0 = lootOf(r.a)
  const loot1 = r.c ? lootOf(r.c) : null
  const keys = new Set([...Object.keys(r.a.payQty), ...Object.keys(r.c?.payQty ?? {}), ...Object.keys(loot0), ...Object.keys(loot1 ?? {})])
  const out: RunRow[] = []
  for (const key of keys) {
    const spent = r.b ? Math.max(0, (r.a.payQty[key] ?? 0) - (r.b.payQty[key] ?? 0)) : 0
    // the paying tab's own loot counts only when it was ticked (Done buying → finish)
    const payLooted = r.payCounts && r.b && r.c ? (r.c.payQty[key] ?? 0) - (r.b.payQty[key] ?? 0) : 0
    const looted = (loot1 ? (loot1[key] ?? 0) - (loot0[key] ?? 0) : 0) + payLooted
    if (!spent && !looted) continue
    const info = r.names[key]
    if (!info) continue
    const spentEach = r.b?.price[key] ?? r.a.price[key] ?? 0
    const lootEach = r.c?.price[key] ?? spentEach
    out.push({
      key,
      name: info.name,
      icon: iconFor({ key, icon: info.icon, ask: info.ask }),
      spent,
      looted,
      each: looted ? lootEach : spentEach,
      value: looted * lootEach - spent * spentEach
    })
  }
  const q = search.value.trim().toLowerCase()
  return out
    .filter(x => !q || x.name.toLowerCase().includes(q))
    .sort((x, y) => Math.abs(y.value) - Math.abs(x.value))
})

// auto-snapshot while a run is running (paced by wealth.rs like every sync)
const clock = shallowRef(Date.now())
const ticker = setInterval(() => {
  clock.value = Date.now()
  const run = activeRun.value
  if (!run || run.phase !== 'running' || busy.value || !signedIn.value) return
  const last = (run.c ?? run.b ?? run.a).t
  if (Date.now() - last >= autoSyncEvery.value * 60_000) void snapshotNow()
}, 1000) // every second: the run's stopwatch
onBeforeUnmount(() => clearInterval(ticker))

// ------------------------------------------------ table
const search = shallowRef('')
const sortKey = shallowRef<'name' | 'tab' | 'qty' | 'each' | 'total'>('total')
const sortDesc = shallowRef(true)
function sortBy (k: typeof sortKey.value) {
  if (sortKey.value === k) sortDesc.value = !sortDesc.value
  else { sortKey.value = k; sortDesc.value = k !== 'name' && k !== 'tab' }
}
function sortIcon (k: string) {
  if (sortKey.value !== k) return 'fas fa-sort ww-dim'
  return sortDesc.value ? 'fas fa-sort-down' : 'fas fa-sort-up'
}

const rows = computed(() => listRows.value)
const sortedRows = computed(() => {
  const q = search.value.trim().toLowerCase()
  const list = q ? rows.value.filter(r => r.name.toLowerCase().includes(q) || r.tabName.toLowerCase().includes(q)) : rows.value.slice()
  const k = sortKey.value
  const dir = sortDesc.value ? -1 : 1
  list.sort((a, b) => {
    if (k === 'name') return a.name.localeCompare(b.name) * dir
    if (k === 'tab') return (a.diff != null ? (a.diff - (b.diff ?? 0)) : a.tabName.localeCompare(b.tabName)) * dir
    const av = k === 'total' ? Math.abs(a.total) : a[k]
    const bv = k === 'total' ? Math.abs(b.total) : b[k]
    return (av - bv) * dir
  })
  return list
})
const shownRows = computed(() => sortedRows.value.slice(0, 250))

// ------------------------------------------------ the graph
const chartMode = shallowRef<'total' | 'hourly'>('total')
const range = shallowRef('7')
const chart = computed(() => {
  const all = wealth.history[league.value] ?? []
  const days = Number(range.value)
  const from = days ? Date.now() - days * 86_400_000 : 0
  // only the ticked tabs: each sync's share of them (syncs from before the
  // per-tab record, or with none of them, are left out)
  const ticked = selectedIds.value
  let pts = all
    .filter(p => p.t >= from && p.tabs && ticked.some(id => p.tabs![id] != null))
    .map(p => ({ t: p.t, v: ticked.reduce((s, id) => s + (p.tabs![id] ?? 0), 0) / rate.value }))
  if (chartMode.value === 'hourly') {
    const per: Array<{ t: number, v: number }> = []
    for (let i = 1; i < pts.length; i++) {
      const hours = (pts[i].t - pts[i - 1].t) / 3_600_000
      if (hours < 1 / 60) continue
      per.push({ t: pts[i].t, v: (pts[i].v - pts[i - 1].v) / hours })
    }
    pts = per
  }
  if (!pts.length) return { points: [] as ChartPoint[], line: '', area: '', ticks: [] as string[], hi: 0, change: null as number | null }
  // time: the whole chosen range (all: from the first sync), up to now
  const now = Date.now()
  const t0 = days ? from : pts[0].t
  const t1 = Math.max(now, pts[pts.length - 1].t)
  // level: from 0 up to a round number above the highest (like 270d), so a
  // rise or a fall shows at its true size
  const max = Math.max(...pts.map(p => p.v), 0)
  const min = Math.min(...pts.map(p => p.v), 0)
  const hi = niceCeil(max * 1.08)
  const lo = min < 0 ? -niceCeil(-min * 1.08) : 0
  const span = hi - lo || 1
  const x = (t: number) => t1 === t0 ? 200 : 4 + (t - t0) / (t1 - t0) * 392
  const y = (v: number) => 116 - (v - lo) / span * 110
  const points: ChartPoint[] = pts.map(p => ({ x: x(p.t), y: y(p.v), t: p.t, v: p.v }))
  const line = points.map((p, i) => `${i ? 'L' : 'M'}${p.x.toFixed(1)},${p.y.toFixed(1)}`).join(' ')
  const base = y(Math.max(lo, 0)).toFixed(1)
  const area = `${line} L${points[points.length - 1].x.toFixed(1)},${base} L${points[0].x.toFixed(1)},${base} Z`
  // five dates (or times) along the bottom
  const short = (t1 - t0) <= 36 * 3_600_000
  const ticks = [0, 1, 2, 3, 4].map(i => {
    const t = t0 + (t1 - t0) * i / 4
    return short
      ? new Date(t).toLocaleTimeString(undefined, { hour: '2-digit', minute: '2-digit' })
      : new Date(t).toLocaleDateString(undefined, { month: 'short', day: 'numeric' })
  })
  return {
    points,
    line,
    area,
    ticks,
    hi,
    change: pts.length > 1 ? pts[pts.length - 1].v - pts[0].v : null
  }
})

interface ChartPoint { x: number, y: number, t: number, v: number }

/** 241 → 270, 8.2 → 9, 0.43 → 0.5: a round top for the graph */
function niceCeil (v: number) {
  if (v <= 0) return 1
  const step = Math.pow(10, Math.floor(Math.log10(v)) - 1)
  const n = Math.ceil(v / step)
  const nice = n <= 10 ? n : n <= 20 ? Math.ceil(n / 2) * 2 : n <= 50 ? Math.ceil(n / 5) * 5 : Math.ceil(n / 10) * 10
  return nice * step
}

// ------------------------------------------------ hovering the graph
const hoverIdx = shallowRef<number | null>(null)
const hovered = computed(() => hoverIdx.value == null ? null : chart.value.points[hoverIdx.value] ?? null)
function hoverChart (e: MouseEvent) {
  const pts = chart.value.points
  if (!pts.length) return
  const r = (e.currentTarget as HTMLElement).getBoundingClientRect()
  const vx = (e.clientX - r.left) / r.width * 400
  let best = 0
  for (let i = 1; i < pts.length; i++) {
    if (Math.abs(pts[i].x - vx) < Math.abs(pts[best].x - vx)) best = i
  }
  hoverIdx.value = best
}
function fmtLong (t: number) {
  return new Date(t).toLocaleString(undefined, { weekday: 'long', month: 'short', day: 'numeric', hour: 'numeric', minute: '2-digit' })
}

// ------------------------------------------------ formatting
function fmtDiv (v: number) {
  const a = Math.abs(v)
  return a >= 100 ? v.toFixed(0) : a >= 10 ? v.toFixed(1) : v.toFixed(2)
}
function fmtValue (chaos: number) {
  const neg = chaos < 0 ? '-' : ''
  const a = Math.abs(chaos)
  if (a >= rate.value * 0.95) return `${neg}${fmtDiv(a / rate.value)}d`
  if (a >= 10) return `${neg}${Math.round(a)}c`
  if (a >= 1) return `${neg}${a.toFixed(1).replace(/\.0$/, '')}c`
  return `${neg}${a.toFixed(2).replace(/0$/, '')}c`
}
function fmtAgo (t: number) {
  const m = Math.round((Date.now() - t) / 60000)
  if (m < 1) return 'just now'
  if (m < 60) return `${m} min`
  if (m < 24 * 60) return `${Math.round(m / 60)} h`
  const d = Math.round(m / 1440)
  return d === 1 ? '1 day' : `${d} days`
}
function fmtDuration (ms: number) {
  const m = Math.floor(ms / 60000)
  if (m < 1) return 'under 1 min'
  return m < 60 ? `${m} min` : `${Math.floor(m / 60)} h ${m % 60} min`
}
/** a wait: 4:59 */
function fmtWait (s: number) {
  const t = Math.ceil(s)
  return `${Math.floor(t / 60)}:${String(t % 60).padStart(2, '0')}`
}

/** a stopwatch: 0:00:00 */
function fmtClock (ms: number) {
  const s = Math.floor(ms / 1000)
  const two = (n: number) => String(n).padStart(2, '0')
  return `${Math.floor(s / 3600)}:${two(Math.floor(s / 60) % 60)}:${two(s % 60)}`
}
function fmtDate (t: number) {
  return new Date(t).toLocaleString(undefined, { month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit' })
}

/** text size: like WeledFTW's Wealthy Weled, 14 px on a 1080p screen, more on bigger ones */
const fontPx = Math.max(11, Math.round(14 * Math.min(screen.height / 1080, screen.width / 1920)))
function resizeWindow (e: MouseEvent) {
  if (e.button !== 0) return
  void invoke('vs_resize').catch(() => {})
  const w = (window as unknown as { __TAURI__?: { window: { getCurrentWindow: () => { startResizeDragging: (d: string) => Promise<void> } } } }).__TAURI__
  void w?.window.getCurrentWindow().startResizeDragging('SouthEast').catch(() => {})
}
function resetSize () {
  void invoke('vs_reset_size').catch(() => {})
}
/** puts the window away (it stays next to the clock; Ctrl+W brings it back) */
function hideWindow () {
  void invoke('vs_hide').catch(() => {})
}
/** dragging the window by its title bar or the sign-in screen */
function dragWindow (e: MouseEvent) {
  if (e.button !== 0) return
  if ((e.target as HTMLElement).closest('button, a, select, input, label')) return
  void invoke('vs_drag').catch(() => {})
}
function onKey (e: KeyboardEvent) {
  const closeKey = e.key === 'Escape' || (e.ctrlKey && !e.shiftKey && !e.altKey && e.code === 'KeyW')
  if (!closeKey) return
  // Esc first closes an open dropdown or leaves a search box
  if (e.key === 'Escape' && document.activeElement instanceof HTMLInputElement && document.activeElement.value) return
  e.preventDefault()
  hideWindow()
}
window.addEventListener('keydown', onKey)
onBeforeUnmount(() => window.removeEventListener('keydown', onKey))

// "made by Waleed": the maker's Path of Exile profile, in the player's browser
const MAKER_PROFILE = 'https://www.pathofexile.com/account/view-profile/Omaewamoushinderuu-0071'
function openMaker () {
  void invoke('vs_open_url', { url: MAKER_PROFILE }).catch(() => {})
}

// updates from the GitHub releases (updater.rs): looked for at start and every
// 6 hours; installing restarts Vaal Street on the new version
const update = shallowRef<{ version: string, notes: string, current: string } | null>(null)
const updating = shallowRef(false)
/** "Later": the notice goes, the title bar button stays */
const updateHidden = shallowRef(false)
const updatePct = shallowRef(0)
const updateStage = shallowRef<'download' | 'install'>('download')
const updateCells = computed(() => Math.round(updatePct.value / 5))
onEvent('update-progress', (e) => {
  if (e.installing) { updateStage.value = 'install'; updatePct.value = 100; return }
  updatePct.value = e.total ? Math.min(100, Math.floor(e.downloaded / e.total * 100)) : Math.min(95, updatePct.value + 1)
})
async function checkUpdate () {
  const r = await invoke<{ available: boolean, version?: string, notes?: string, current: string }>('vs_update_check').catch(() => null)
  update.value = r?.available ? { version: r.version ?? '', notes: r.notes ?? '', current: r.current } : null
}
setTimeout(() => { void checkUpdate() }, 5000)
const updateTimer = setInterval(() => { void checkUpdate() }, 6 * 3600_000)
onBeforeUnmount(() => clearInterval(updateTimer))
async function installUpdate () {
  updating.value = true
  updateHidden.value = false
  updatePct.value = 0
  updateStage.value = 'download'
  try {
    await invoke('vs_update_install')
  } catch (e) {
    error.value = `Update failed: ${String(e)}`
    updating.value = false
  }
}

// "Startup on/off": Vaal Street in the user's Windows Run list (startup.rs)
const startupOn = shallowRef(false)
const startupBusy = shallowRef(false)
void invoke<boolean>('vs_startup_get').then(v => { startupOn.value = v }).catch(() => {})
async function toggleStartup () {
  startupBusy.value = true
  try {
    startupOn.value = await invoke<boolean>('vs_startup_set', { on: !startupOn.value })
  } catch (e) {
    error.value = `Could not change Startup: ${String(e)}`
  } finally {
    startupBusy.value = false
  }
}

// prices stay fresh while the program is open (poe.ninja keeps them 30 min)
const priceTimer = setInterval(() => ninja.queuePricesFetch(), 10 * 60_000)
onBeforeUnmount(() => clearInterval(priceTimer))

// last: it runs right away and uses everything above
;(() => {
  ninja.queuePricesFetch()
  void loadMapIcons(league.value)
  void loadWealth().then(async () => {
    // ask again every time: the saved sign-in may have been restored since
    if (!signedIn.value) {
      checking.value = true
      await recheckSession()
      checking.value = false
    }
    await readAccount()
    if (ready.value && league.value && !(wealth.tabs[league.value]?.length)) void refreshTabs()
  })
})()
// the league list can arrive after the window opened, or the player picks another
watch(league, (id) => {
  ninja.queuePricesFetch()
  void loadMapIcons(id)
  if (id && ready.value && !(wealth.tabs[id]?.length)) void refreshTabs()
})
</script>

<style scoped>
/* ------------------------------- Vaal Street: a see-through window over the game */
.vs-page {
  position: fixed; inset: 0; display: flex; padding: 14px; overflow: hidden;
  background: transparent;
}
.ww-grip {
  position: absolute; right: 0; bottom: 0; z-index: 60; width: 18px; height: 18px; cursor: nwse-resize;
  background:
    linear-gradient(var(--cyan), var(--cyan)) 12px 12px / 3px 3px,
    linear-gradient(var(--cyan), var(--cyan)) 7px 12px / 3px 3px,
    linear-gradient(var(--cyan), var(--cyan)) 12px 7px / 3px 3px,
    linear-gradient(var(--cyan), var(--cyan)) 2px 12px / 3px 3px,
    linear-gradient(var(--cyan), var(--cyan)) 7px 7px / 3px 3px,
    linear-gradient(var(--cyan), var(--cyan)) 12px 2px / 3px 3px;
  background-repeat: no-repeat; opacity: .55;
}
.ww-grip:hover { opacity: 1; filter: drop-shadow(0 0 4px var(--accent-glow)); }
.vs-close-corner { position: absolute; top: 0.2em; right: 0.2em; z-index: 60; }
.ww-signin { padding-top: 3.4em; }
.vs-page > .ww-root { flex: 1; display: flex; flex-direction: column; min-height: 0; min-width: 0; }
.vs-page .ww-body { flex: 1; height: auto; min-height: 9em; }
/* before the sign-in: the logo, the button, the maker, in the middle */
.vs-page-signin { align-items: center; justify-content: center; }
.vs-page-signin > .ww-root { flex: 0 0 auto; width: min(680px, 92vw); min-width: 0; }
.vs-page .ww-signin { cursor: default; }
.vs-head { cursor: default; }
.vs-made-by {
  margin-top: -0.4em; font-family: var(--font); -webkit-font-smoothing: none;
  font-size: 1.35em; letter-spacing: 0.25em; text-transform: uppercase; color: var(--muted);
}
.vs-maker { color: var(--cyan); text-decoration: none; border-bottom: 1px dashed var(--cyan); cursor: pointer; }
.vs-maker:hover { color: var(--accent-ink); background: var(--cyan); border-bottom-color: transparent; }
.vs-league { font-size: 0.95em; padding: 0.2em 0.4em; }
/* ---------------------------------------------- the update notice (pixel art) */
.vs-upd {
  position: fixed; top: 4.6em; right: 1.6em; z-index: 200; width: 25em;
  display: flex; gap: 1em; padding: 1em 1.1em 1.1em;
  font-family: var(--text-font);
}
.vs-drop-enter-active { animation: vs-drop-in 0.55s steps(7) both; }
.vs-drop-leave-active { animation: vs-drop-in 0.3s steps(4) reverse both; }
@keyframes vs-drop-in {
  0% { transform: translateY(-160%); opacity: 0; }
  70% { transform: translateY(6%); opacity: 1; }
  100% { transform: none; opacity: 1; }
}
/* the art: a download arrow over a tray, drawn from 5 px squares */
.vs-upd-art { position: relative; flex: none; width: 35px; height: 45px; margin-top: 0.3em; }
.vs-px-arrow, .vs-px-tray { position: absolute; left: 0; top: 0; width: 5px; height: 5px; background: transparent; }
.vs-px-arrow {
  color: var(--ink);
  box-shadow:
    15px 0 0 currentColor, 15px 5px 0 currentColor, 15px 10px 0 currentColor, 15px 15px 0 currentColor,
    5px 20px 0 currentColor, 10px 20px 0 currentColor, 15px 20px 0 currentColor, 20px 20px 0 currentColor, 25px 20px 0 currentColor,
    10px 25px 0 currentColor, 15px 25px 0 currentColor, 20px 25px 0 currentColor,
    15px 30px 0 currentColor;
  /* the wordmark's pink and cyan fringe */
  filter: drop-shadow(-2px 2px 0 #ff5d8f) drop-shadow(2px -2px 0 #4de3ff);
  animation: vs-bob 0.9s steps(2) infinite;
}
.vs-px-tray {
  color: var(--gold);
  box-shadow:
    0 35px 0 currentColor, 30px 35px 0 currentColor,
    0 40px 0 currentColor, 5px 40px 0 currentColor, 10px 40px 0 currentColor, 15px 40px 0 currentColor,
    20px 40px 0 currentColor, 25px 40px 0 currentColor, 30px 40px 0 currentColor;
  filter: drop-shadow(0 0 4px color-mix(in srgb, var(--gold) 70%, transparent));
}
@keyframes vs-bob { 50% { transform: translateY(5px); } }
/* downloading: the arrow keeps falling into the tray */
.vs-upd-art.busy .vs-px-arrow { animation: vs-fall 0.6s steps(4) infinite; }
@keyframes vs-fall {
  0% { transform: translateY(-15px); opacity: 0; }
  25% { opacity: 1; }
  100% { transform: translateY(5px); opacity: 1; }
}
.vs-upd-body { flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 0.45em; }
.vs-upd-title { font-size: 2em; line-height: 1; }
.vs-upd-ver { display: flex; align-items: baseline; gap: 0.5em; font-family: var(--font); -webkit-font-smoothing: none; font-size: 1.5em; letter-spacing: 0.06em; }
.vs-upd-old { color: var(--muted); text-decoration: line-through; }
.vs-upd-to { color: var(--cyan); font-size: 0.7em; animation: vs-blink 1s steps(2) infinite; }
.vs-upd-ver b { color: var(--gold); font-weight: normal; text-shadow: 0 0 8px color-mix(in srgb, var(--gold) 60%, transparent); }
@keyframes vs-blink { 50% { opacity: 0.2; } }
.vs-upd-notes { font-size: 0.85em; color: var(--muted); max-height: 5.2em; overflow: auto; white-space: pre-line; }
.vs-upd-btns { display: flex; gap: 0.5em; margin-top: 0.2em; }
.vs-upd-btns button { height: 2.3em; padding: 0 0.9em; display: inline-flex; align-items: center; gap: 0.4em; }
.vs-upd-go { flex: 1; justify-content: center; }
/* the progress: 20 blocks lighting up one by one */
.vs-upd-bar { display: grid; grid-template-columns: repeat(20, 1fr); gap: 2px; padding: 3px; border: 1px solid var(--line-strong); background: var(--field); }
.vs-upd-bar span { height: 0.95em; background: color-mix(in srgb, var(--cyan) 10%, transparent); transition: background-color 0.1s steps(1); }
.vs-upd-bar span.on { background: var(--cyan); box-shadow: 0 0 6px var(--accent-glow); }
.vs-upd-pct { font-family: var(--font); -webkit-font-smoothing: none; font-size: 1.3em; letter-spacing: 0.1em; color: var(--cyan); }

.vs-update {
  display: inline-flex; align-items: center; gap: 0.4em; height: 2em; padding: 0 0.8em; font-weight: 600;
  color: var(--accent-ink); background: var(--gold); border: 1px solid var(--gold);
  box-shadow: 0 0 12px color-mix(in srgb, var(--gold) 50%, transparent);
  animation: vs-glow 2s steps(4) infinite;
}
.vs-update:disabled { opacity: 0.7; animation: none; }
@keyframes vs-glow { 50% { box-shadow: 0 0 2px color-mix(in srgb, var(--gold) 30%, transparent); } }
.vs-startup {
  display: inline-flex; align-items: center; gap: 0.4em; height: 2em; padding: 0 0.7em;
  font-size: 0.95em; color: var(--muted); background: var(--field); border: 1px solid var(--line-strong);
  transition: color .12s steps(2), border-color .12s steps(2), box-shadow .12s steps(2);
}
.vs-startup b { font-weight: 700; padding: 0 0.35em; color: var(--muted); border: 1px solid var(--line-strong); }
.vs-startup:hover:not(:disabled) { color: var(--cyan); border-color: var(--cyan); }
.vs-startup.on { color: var(--good); border-color: color-mix(in srgb, var(--good) 60%, transparent); }
.vs-startup.on b { color: var(--accent-ink); background: var(--good); border-color: var(--good); }
.vs-startup:disabled { opacity: 0.6; }

/* The filter manager's look (weled.css / tokens.css): square frame with
   corner ticks over deep water (dark) or the sunlit shallows (light), pixel
   wordmark, "// HEADINGS", cyan accents. Every colour is a theme token. */
.ww-root {
  padding: 0.9em 1.1em 1em;
  font-family: var(--text-font);
  animation: ww-pop 0.22s steps(4) both;
}
@keyframes ww-pop { from { opacity: 0; transform: translateY(-0.6em) } to { opacity: 1; transform: none } }

.ww-head { display: flex; align-items: center; justify-content: space-between; gap: 1em; margin-bottom: 0.8em; cursor: grab; user-select: none; }
.ww-head.ww-dragging { cursor: grabbing; }
.ww-brand { display: flex; align-items: baseline; gap: 0.9em; }
.ww-logo { font-size: 2.3em; }
.ww-tag { font-size: 0.95em; }
.ww-sub { display: flex; align-items: center; gap: 0.8em; font-size: 0.85em; }
.ww-acc { color: var(--ink); white-space: nowrap; }
.ww-last { color: var(--muted); white-space: nowrap; }
.ww-last .fas { margin-right: 0.2em; }
/* more than an hour old: a nudge to sync */
.ww-last.old { color: var(--warn); }
.ww-dim { color: var(--muted); }
.ww-tool { width: 2em; height: 2em; }
/* the title bar wraps onto a second line instead of pushing buttons out */
.ww-head { flex-wrap: wrap; row-gap: 0.5em; }
.ww-brand { flex: none; }
.ww-sub { flex-wrap: wrap; justify-content: flex-end; row-gap: 0.45em; margin-left: auto; min-width: 0; }
.ww-sub > * { flex: none; white-space: nowrap; }
.ww-toolset { display: flex; align-items: center; gap: 0.4em; }

.ww-btn { gap: 0.45em; padding: 0.32em 0.9em; }
.ww-btn:disabled { opacity: 0.45; cursor: default; }
.wf-primary.ww-btn:disabled:hover { background: var(--accent-soft); color: var(--cyan); }
.ww-sync { max-width: 30em; overflow: hidden; white-space: nowrap; text-overflow: ellipsis; }
.ww-stop:hover:not(:disabled) { border-color: var(--pink); color: var(--pink); }

/* -------------------------------------------------- sign in */
.ww-signin { position: relative; display: flex; flex-direction: column; align-items: center; gap: 1.6em; padding: 3em 1em 2.6em; text-align: center; cursor: grab; user-select: none; }
.ww-corner { position: absolute; top: -0.2em; right: -0.3em; }
.ww-big-logo { font-size: 5.2em; }
.ww-signin-btn { font-size: 1.2em; padding: 0.6em 2.2em; cursor: pointer; }

.ww-error { margin-bottom: 0.6em; padding: 0.45em 0.8em; color: var(--bad); background: color-mix(in srgb, var(--bad) 12%, transparent); border: 1px solid var(--bad); }

/* -------------------------------------------------- top cards */
.ww-top { display: grid; grid-template-columns: 1fr 1fr; gap: 0.8em; margin-bottom: 0.8em; }
.ww-card { padding: 0.7em 0.9em; background: var(--surface); border: 1px solid var(--line); box-shadow: var(--in); }
.ww-card-head { display: flex; align-items: center; gap: 0.7em; margin-bottom: 0.5em; }
.ww-card-title { font-size: 1.25em; margin-right: auto; }
.ww-net { font-size: 1.5em; color: var(--gold); display: flex; align-items: center; gap: 0.25em; font-weight: 600; }
.ww-cur { width: 1.1em; height: 1.1em; display: inline-block; vertical-align: middle; }
.ww-seg { display: inline-flex; border: 1px solid var(--line-strong); background: var(--field); }
.ww-seg button { padding: 0.2em 0.7em; font-size: 0.85em; color: var(--muted); transition: color .12s steps(2), background-color .12s steps(2); }
.ww-seg button:hover { color: var(--cyan); }
.ww-seg button.on { color: var(--accent-ink); background: var(--cyan); font-weight: 600; }
.ww-seg-wide { display: flex; margin-bottom: 0.5em; }
.ww-seg-wide button { flex: 1; padding: 0.4em; font-family: var(--font); font-size: 1.15em; letter-spacing: 0.15em; text-transform: uppercase; -webkit-font-smoothing: none; }
.ww-select { padding: 0.15em 0.4em; font-size: 0.85em; }
/* -------------------------------------------------- runs */
.ww-run { display: flex; flex-direction: column; gap: 0.5em; padding: 0.7em; background: var(--surface); border: 1px solid var(--line-strong); }
.ww-steps { display: flex; gap: 0.25em; }
.ww-step {
  flex: 1; display: flex; align-items: center; justify-content: center; gap: 0.3em; padding: 0.2em 0;
  font-family: var(--font); -webkit-font-smoothing: none; font-size: 1.05em; letter-spacing: 0.08em; text-transform: uppercase;
  color: var(--faint); border: 1px solid var(--line); background: var(--field);
}
.ww-step b { font-weight: normal; color: inherit; opacity: 0.8; }
.ww-step.past { color: var(--muted); border-color: var(--line-strong); }
.ww-step.on { color: var(--accent-ink); background: var(--cyan); border-color: var(--cyan); box-shadow: 0 0 12px var(--accent-glow); }
.ww-label { font-size: 0.78em; color: var(--muted); text-transform: uppercase; letter-spacing: 0.08em; margin-top: 0.2em; }
.ww-small-text { font-size: 0.82em; }
.ww-paytab {
  display: flex; align-items: center; gap: 0.45em; padding: 0.3em 0.5em;
  background: var(--tab); color: var(--tab-ink); font-family: var(--text-font); font-size: 1.08em;
}
.ww-paytab img { width: 1.5em; height: 1.5em; object-fit: contain; }
.ww-paytab span { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.ww-change-pay { font-family: var(--text-font); font-size: 0.75em; padding: 0.1em 0.5em; color: var(--tab-ink); background: rgba(0, 0, 0, 0.25); }
.ww-pay-select { width: 100%; padding: 0.3em; }
.ww-big-btn { justify-content: center; padding: 0.5em 1em; font-size: 1.05em; margin-top: 0.3em; }
.ww-grow { flex: 1; }
/* Check and Finish: the same size, side by side */
.ww-pair { display: grid; grid-template-columns: 1fr 1fr; gap: 0.5em; }
.ww-pair .ww-btn { justify-content: center; height: 2.4em; padding: 0 0.6em; font-size: 1em; }
.ww-alert {
  display: flex; align-items: flex-start; gap: 0.55em; padding: 0.45em 0.6em; font-size: 0.85em; line-height: 1.35;
  color: var(--ink); border: 1px solid color-mix(in srgb, var(--warn) 60%, transparent);
  background: color-mix(in srgb, var(--warn) 12%, transparent);
}
.ww-alert > .fas { color: var(--warn); margin-top: 0.15em; }
.ww-warn-text { color: var(--warn); }
.ww-pay-note { margin-top: 0.25em; color: var(--faint); }
.ww-alert b { color: var(--warn); }
.ww-run-what { display: flex; flex-direction: column; }
.ww-watch {
  font-family: var(--font); -webkit-font-smoothing: none; font-size: 2.6em; line-height: 1; text-align: center;
  letter-spacing: 0.06em; color: var(--cyan); text-shadow: 0 0 12px var(--accent-glow); font-variant-numeric: tabular-nums;
}
.ww-phase { display: flex; align-items: flex-start; gap: 0.6em; padding: 0.6em 0.7em; font-size: 0.92em; line-height: 1.35; border: 1px solid; }
.ww-phase > .fas { font-size: 1.5em; margin-top: 0.1em; }
.ww-phase-buy { color: var(--gold); border-color: color-mix(in srgb, var(--gold) 55%, transparent); background: color-mix(in srgb, var(--gold) 10%, transparent); }
.ww-phase-buy div { color: var(--ink); }
.ww-phase-buy b { color: var(--gold); }
.ww-phase-run { color: var(--ink); border-color: color-mix(in srgb, var(--pink) 50%, transparent); background: color-mix(in srgb, var(--pink) 8%, transparent); align-items: center; }
.ww-phase-busy { color: var(--cyan); border-color: var(--line-strong); background: var(--field); align-items: center; }
.ww-live { margin-left: 0.3em; padding: 0 0.35em; font-size: 0.85em; color: var(--accent-ink); background: var(--pink); }

.ww-report-head { display: flex; align-items: baseline; justify-content: space-between; gap: 1em; margin-bottom: 0.5em; }
.ww-report-head .wf-heading { font-size: 1.2em; }
.ww-cards { display: grid; grid-template-columns: repeat(6, minmax(0, 1fr)); gap: 0.5em; margin-bottom: 0.6em; }
.ww-stat { display: flex; flex-direction: column; gap: 0.15em; padding: 0.45em 0.6em; background: var(--surface); border: 1px solid var(--line); box-shadow: var(--in); }
.ww-stat span { font-size: 0.75em; color: var(--muted); text-transform: uppercase; letter-spacing: 0.08em; }
.ww-stat b { font-size: 1.2em; color: var(--ink); display: flex; align-items: center; gap: 0.25em; white-space: nowrap; }
.ww-stat.neg b { color: var(--bad); }
.ww-stat-cost b { color: var(--bad); }
.ww-stat-rev b { color: var(--good); }
.ww-stat-profit { border-color: var(--good); background: color-mix(in srgb, var(--good) 10%, var(--surface)); }
.ww-stat-profit b { color: var(--good); font-size: 1.45em; }
.ww-stat-profit.neg { border-color: var(--bad); background: color-mix(in srgb, var(--bad) 10%, var(--surface)); }
.ww-stat-profit.none { border-color: var(--line); background: var(--surface); }
.ww-stat-profit.none b { color: var(--muted); }
.ww-tr.ww-run-tr { grid-template-columns: minmax(0, 2.6fr) repeat(4, minmax(0, 0.9fr)); }
.ww-how { text-align: left; max-width: 34em; margin: 2em auto; }
.ww-how ol { margin: 0.8em 0 0 1.2em; list-style: decimal; display: flex; flex-direction: column; gap: 0.5em; color: var(--ink); }

.ww-chart-wrap { position: relative; cursor: crosshair; }
.ww-chart { width: 100%; height: 7.5em; display: block; border-left: 1px solid var(--line-strong); border-bottom: 1px solid var(--line-strong); }
.ww-y-top { position: absolute; left: 0.35em; top: 0; font-size: 0.78em; color: var(--muted); pointer-events: none; }
.ww-guide { stroke: var(--line-strong); stroke-width: 1; stroke-dasharray: 3 3; }
.ww-pt {
  position: absolute; width: 0.5em; height: 0.5em; margin: -0.25em 0 0 -0.25em; border-radius: 50%;
  background: var(--ink); box-shadow: 0 0 0 1.5px var(--cyan); pointer-events: none;
}
.ww-pt.on { width: 0.8em; height: 0.8em; margin: -0.4em 0 0 -0.4em; background: var(--gold); box-shadow: 0 0 0 2px var(--cyan), 0 0 10px var(--accent-glow); }
.ww-tip {
  position: absolute; z-index: 5; transform: translate(0.8em, -50%); pointer-events: none; white-space: nowrap;
  padding: 0.4em 0.7em; background: var(--solid); border: 1px solid var(--line-strong); box-shadow: var(--float); color: var(--ink); font-size: 0.88em;
}
.ww-tip.flip { transform: translate(calc(-100% - 0.8em), -50%); }
.ww-tip-date { color: var(--muted); }
.ww-tip-val { color: var(--gold); }
.ww-change { font-weight: 600; color: var(--good); }
.ww-change.neg { color: var(--bad); }
.ww-axis-empty { flex: 1; text-align: center; }
.ww-line { stroke: var(--cyan); filter: drop-shadow(0 0 3px var(--accent-glow)); }
.ww-dot { fill: var(--ink); stroke: var(--cyan); stroke-width: 1.5; }
.ww-chart-axis { display: flex; justify-content: space-between; font-size: 0.78em; color: var(--muted); margin-top: 0.2em; }

.ww-bd-grid { display: grid; grid-template-columns: repeat(3, minmax(0, 1fr)); gap: 0.45em 0.7em; }
.ww-bd { display: flex; align-items: center; gap: 0.4em; min-width: 0; padding: 0.3em 0.45em; background: var(--field); border: 1px solid var(--line); }
.ww-bd-dot { width: 0.7em; height: 0.7em; flex: none; display: inline-block; margin-right: 0.35em; box-shadow: 0 0 0 1px var(--line-strong); }
.ww-bd .ww-bd-dot { margin-right: 0; }
.ww-bd { border-left: 3px solid var(--tab); }
.ww-bd-icon { width: 1.6em; height: 1.6em; object-fit: contain; flex: none; }
.ww-cell-icon { width: 1.3em; height: 1.3em; object-fit: contain; flex: none; margin-right: 0.35em; }
.ww-cell-tab { overflow: hidden; text-overflow: ellipsis; padding-left: 0.35em; border-left: 3px solid var(--tab); }
.ww-bd-name { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-size: 0.88em; color: var(--muted); }
.ww-bd-val { color: var(--ink); font-weight: 600; }

/* -------------------------------------------------- body */
.ww-body { display: grid; grid-template-columns: 17em minmax(0, 1fr); gap: 0.8em; height: 30em; }
.ww-side { display: flex; flex-direction: column; min-height: 0; }
.ww-tabs { flex: 1; overflow-y: auto; border: 1px solid var(--line-strong); background: var(--field); }
/* every tab in the colour it has in the stash (pathofexile.com), like the game */
.ww-tab {
  display: flex; align-items: center; gap: 0.45em; padding: 0.2em 0.5em; cursor: pointer;
  background: var(--tab); color: var(--tab-ink);
  border-bottom: 1px solid rgba(0, 0, 0, 0.35);
  /* not ticked: the same colour, only a touch quieter (the box shows it) */
  opacity: 0.88;
  transition: opacity .12s steps(2), filter .12s steps(2);
}
.ww-tab:hover { opacity: 1; filter: brightness(1.12); }
.ww-tab.on { opacity: 1; }
.ww-tab-check { width: 1.4em; font-size: 0.95em; padding: 0.2em; margin: -0.2em; cursor: pointer; text-align: center; }
.ww-tab-check:hover { transform: scale(1.2); }
/* the tab whose items are shown */
.ww-tab.viewing { opacity: 1; box-shadow: inset 0 0 0 2px var(--cyan), inset 4px 0 0 var(--cyan); }
.ww-bd { cursor: pointer; }
.ww-bd:hover { border-color: var(--cyan); }
.ww-bd.viewing { border-color: var(--cyan); box-shadow: var(--lift); }
.ww-viewing {
  display: inline-flex; align-items: center; gap: 0.4em; padding: 0.15em 0.6em; font-weight: 600;
  background: var(--tab); color: var(--tab-ink); border: 1px solid var(--cyan);
}
.ww-viewing img { width: 1.3em; height: 1.3em; object-fit: contain; }
.ww-viewing .fa-times { opacity: 0.8; }
.ww-viewing:hover .fa-times { opacity: 1; }
.ww-tab.on .ww-tab-check { color: var(--tab-ink); }
.ww-tab-name { text-shadow: none; }
.ww-tab-icon { width: 1.55em; height: 1.55em; object-fit: contain; flex: none; filter: drop-shadow(0 0 2px rgba(0, 0, 0, 0.7)); }
.ww-tab-icon-fa { display: grid; place-items: center; font-size: 0.95em; }
.ww-tab-name {
  flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
  font-family: var(--text-font); font-size: 1.12em; letter-spacing: 0.01em;
}
.ww-tab-na { font-size: 0.72em; padding: 0 0.35em; color: #ffd166; background: rgba(0, 0, 0, 0.55); white-space: nowrap; cursor: help; }
.ww-tab-val { font-size: 0.8em; font-weight: 700; padding: 0 0.35em; color: #ffd166; background: rgba(0, 0, 0, 0.55); }
.ww-side-foot { display: flex; justify-content: space-between; align-items: center; gap: 0.5em; margin-top: 0.4em; }
.ww-count { font-size: 0.82em; color: var(--muted); font-variant-numeric: tabular-nums; }
.ww-count.full { color: var(--warn); font-weight: 600; }
.ww-cap-note { margin-top: 0.4em; padding: 0.3em 0.5em; font-size: 0.8em; color: var(--warn); border: 1px solid color-mix(in srgb, var(--warn) 50%, transparent); background: color-mix(in srgb, var(--warn) 10%, transparent); }
.ww-link { color: var(--cyan); font-size: 0.88em; }
.ww-link:hover { text-decoration: underline; }
.ww-pad { padding: 0.6em; }

.ww-snap-now { display: flex; flex-direction: column; gap: 0.45em; padding: 0.7em; background: var(--surface); border: 1px solid var(--line-strong); font-size: 0.9em; }
.ww-snap-live { display: flex; align-items: center; gap: 0.45em; color: var(--ink); }
.ww-rec { width: 0.6em; height: 0.6em; background: var(--pink); box-shadow: 0 0 8px var(--pink); animation: ww-blink 1.2s steps(2) infinite; }
@keyframes ww-blink { 50% { opacity: 0.25 } }
.ww-snap-profit { font-size: 2em; color: var(--good); display: flex; align-items: center; gap: 0.25em; font-weight: 600; }
.ww-snap-profit.neg, .ww-snap-val.neg { color: var(--bad); }
.ww-row { display: flex; gap: 0.5em; }
.ww-start { align-self: flex-start; }
.ww-snaps { flex: 1; overflow-y: auto; margin-top: 0.5em; display: flex; flex-direction: column; gap: 0.35em; }
.ww-snap { position: relative; padding: 0.45em 0.6em; cursor: pointer; background: var(--field); border: 1px solid var(--line); }
.ww-snap:hover { border-color: var(--line-strong); }
.ww-snap.on { border-color: var(--cyan); box-shadow: var(--lift); }
.ww-snap-when { font-size: 0.8em; color: var(--muted); }
.ww-snap-val { color: var(--good); font-weight: 600; }
.ww-x { position: absolute; right: 0.5em; top: 0.5em; color: var(--muted); font-size: 0.8em; opacity: 0; }
.ww-snap:hover .ww-x { opacity: 1; }
.ww-x:hover { color: var(--pink); }

.ww-main { display: flex; flex-direction: column; min-height: 0; }
.ww-tools { display: flex; align-items: center; gap: 0.8em; margin-bottom: 0.45em; }
.ww-search { position: relative; display: flex; align-items: center; width: 16em; }
.ww-search .fas { position: absolute; left: 0.6em; color: var(--muted); font-size: 0.85em; }
.ww-search input { width: 100%; padding: 0.3em 0.6em 0.3em 2em; outline: none; }
.ww-table { flex: 1; min-height: 0; display: flex; flex-direction: column; border: 1px solid var(--line-strong); background: var(--field); }
.ww-rows { flex: 1; overflow-y: auto; }
.ww-tr { display: grid; grid-template-columns: minmax(0, 2.6fr) minmax(0, 1.3fr) 0.8fr 0.8fr 0.8fr; align-items: center; gap: 0.6em; padding: 0.28em 0.7em; border-bottom: 1px solid var(--line); color: var(--ink); }
.ww-rows .ww-tr:nth-child(even) { background: var(--stripe); }
.ww-rows .ww-tr:hover { background: var(--hover); }
.ww-th { font-family: var(--font); -webkit-font-smoothing: none; font-size: 1.1em; letter-spacing: 0.12em; text-transform: uppercase; color: var(--cyan); background: var(--surface); user-select: none; border-bottom: 1px solid var(--line-strong); }
.ww-th span { cursor: pointer; }
.ww-th .ww-dim { opacity: 0.5; }
.c-name { display: flex; align-items: center; gap: 0.5em; min-width: 0; }
.c-tab { display: flex; align-items: center; min-width: 0; overflow: hidden; white-space: nowrap; text-overflow: ellipsis; color: var(--muted); font-size: 0.9em; }
.ww-th .c-tab { font-size: 1em; color: var(--cyan); }
.c-num { text-align: right; font-variant-numeric: tabular-nums; }
.ww-icon { width: 1.7em; height: 1.7em; object-fit: contain; flex: none; }
/* no picture: the same space, so every name lines up */
.ww-icon-empty { display: inline-block; }
.ww-name { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.ww-total { color: var(--gold); font-weight: 600; }
.up { color: var(--good); font-weight: 600; }
.down { color: var(--bad); }
.ww-empty { padding: 2.5em 1em; text-align: center; color: var(--muted); }
</style>
