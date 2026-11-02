# Orange Pack — where the samples come from

The thirty one-shots in this folder are Orange Studio's factory sample pack, written once into the
user folder's `Sounds/Orange Pack` and listed in the Browser as **Orange Pack**. They are curated
hits from the **Virtuosity Drums** library by Versilian Studios (with Karoryfer Samples), recorded
at Virtuosity Musical Instruments in Boston, MA.

- Source: https://github.com/studiorack/virtuosity-drums (release v0.9.23, `virtuosity-drums-compact.zip`)
- Upstream page: https://vis.versilstudios.com/virtuosity-drums.html
- License: **Creative Commons Zero 1.0 Universal** (the library's `LICENSE` file), the same
  dedication the repo's own work carries: the samples ship with the app under GPL 3.0-or-later
  without any obligation beyond CC0's, and credit is given anyway.

## The picks

| File in this folder | Source file (under the zip's `Samples/`) |
| --- | --- |
| Orange Kick.wav | snaremic/kick/snaremic_kick_snoff_vl3_rr1.ogg |
| Orange Kick Soft.wav | snaremic/kick/snaremic_kick_snoff_vl1_rr1.ogg |
| Orange Snare.wav | snaremic/snare/snaremic_snare_center_vl15.ogg |
| Orange Snare Off Center.wav | snaremic/snare/snaremic_snare_offcenter_vl15.ogg |
| Orange Rim Shot.wav | snaremic/snare/snaremic_snare_rimshot_vl5.ogg |
| Orange Cross Stick.wav | snaremic/snare/snaremic_snare_crossstick_vl8.ogg |
| Orange Snare Flam.wav | snaremic/snare/snaremic_snare_flam_vl4.ogg |
| Orange Hat Closed.wav | snaremic/hh/snaremic_hh_closed_vl3_rr1.ogg |
| Orange Hat Open.wav | snaremic/hh/snaremic_hh_open_vl3_rr1.ogg |
| Orange Hat Pedal.wav | snaremic/hh/snaremic_hh_pedal_vl3_rr1.ogg |
| Orange Crash Ride.wav | snaremic/crash/snaremic_crash_crash_vl2_rr1.ogg |
| Orange Ride.wav | snaremic/ride/snaremic_ride_ride_vl2_rr1.ogg |
| Orange Ride Bell.wav | snaremic/ride/snaremic_ride_bell_vl2_rr1.ogg |
| Orange Tom High.wav | snaremic/htom/snaremic_htom_center_vl8.ogg |
| Orange Tom Low.wav | snaremic/ltom/snaremic_ltom_center_vl8.ogg |
| Orange Cowbell.wav | perc/close/cowbell/Cowbell1_Normal_v1_rr1_Close.wav |
| Orange Claves.wav | perc/close/claves/Claves1_Hit_v1_rr1_Close.wav |
| Orange Shaker.wav | perc/close/shaker/LShaker_Shake1D_rr1_Close.wav |
| Orange Tambourine.wav | perc/close/tambourine/Tamb1_Shake_rr1_Close.wav |
| Orange Triangle.wav | perc/close/triangle/Triangle1_HitM_v1_rr1_Close.wav |
| Orange Conga.wav | perc/close/conga/Conga_17_HitHM1_1_70_rr1_Close.wav |
| Orange Bongo High.wav | perc/close/bongoh/BongoH_Hit1_v1_rr1_Close.wav |
| Orange Bongo Low.wav | perc/close/bongol/BongoL_Hit1_v1_rr1_Close.wav |
| Orange Cabasa.wav | perc/close/cabasa/Cabasa1_Rub_v1_rr1_Close.wav |
| Orange Vibraslap.wav | perc/close/vibraslap/Vibraslap1_1_Hit_1_70_rr1_Close.wav |
| Orange Agogo.wav | perc/close/agogo/Agogo_High_v1_rr1_Close.wav |
| Orange Guiro.wav | perc/close/guiro/Guiro_Fast_rr1_Close.wav |
| Orange Sleighbells.wav | perc/close/sleighbells/Sleighbells_Hit_rr1_Close.wav |
| Orange Woodblock.wav | perc/oh/woodblock/woodblock_hi_vl1_rr1.wav |
| Orange Bell Tree.wav | perc/oh/belltree/BellTree_Stroke_rr1_Overhead.wav |

The drum hits are the snare-mic set's single hits (one round-robin, one velocity each), the
percussion the close mic's (the woodblock and bell tree the overheads', the only mics that
recorded them). Every file was decoded to mono 44.1 kHz, trimmed of leading and trailing
silence (with 2 ms of air before the hit and 30 ms of room after it, the last 128 samples faded
so a cut never clicks), and peak-normalized to about -1.4 dBFS, as 16-bit PCM WAV.
