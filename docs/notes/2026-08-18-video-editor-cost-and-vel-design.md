> From: https://chatgpt.com/c/6a843018-a9b0-83e8-805d-d56410b44b69

# you asked

message time: 2026-08-18 19:13:10

動画編集ソフトを一からコーディングエージェントで作ろうとしたらどれくらいどんなコストがかかると思う？

---

# chatgpt response

かなり面白い題材よ。動画編集ソフトは「コード量」より、**時間軸・プレビュー・エンコード・GPU・巨大ファイル・Undo/Redo**あたりが急に牙を剥くので、どこまでを自作するかで桁が変わるわ。

## 事実

「動画編集ソフトを一から作る」を、**コーデックまで自作する**意味に取るとほぼ無謀だけれど、**FFmpeg等をレンダリング基盤として、自分の編集UI・タイムライン・プロジェクト形式を作る**なら、コーディングエージェント時代にはかなり現実的になっているわ。

ただし難所はコード生成そのものではない。動画編集では、主にこのへんがコストになる。

| 領域 | 難度 | エージェント適性 | 面倒な理由 |
|---|---:|---:|---|
| ファイルImport/Metadata | 低 | ◎ | ffprobe等で済む |
| Trim / Split / Move | 低〜中 | ◎ | タイムラインモデルが必要 |
| FFmpeg Export | 中 | ◎ | filtergraph生成 |
| 字幕 | 低〜中 | ◎ | SRT/ASS処理は容易 |
| プロジェクト保存 | 低 | ◎ | JSON等でよい |
| Undo / Redo | 中 | ◎ | 状態管理設計が重要 |
| 波形表示 | 中 | ○ | キャッシュ処理 |
| サムネイル生成 | 中 | ◎ | 非同期処理 |
| リアルタイムPreview | **高** | △ | 最大の難所 |
| 複数トラック | 高 | ○ | 時間座標と合成 |
| 音声同期 | 高 | △ | timestamp地獄 |
| Seek | 高 | △ | GOP/keyframe |
| GPU Decode/Encode | 高 | △ | OS/GPU差異 |
| トランジション | 中〜高 | ○ | PreviewとExportの一致 |
| エフェクト | 高 | ○ | レンダリンググラフ |
| Premiere級UI | **非常に高** | △ | interactionの塊 |
| DaVinci級Color | 狂気 | × | 別製品レベル |

だから、例えば

**Electron/Tauri + TypeScript/Rust + FFmpeg**

みたいな構成なら、実態としては

> 「動画編集エンジンを作る」

というより、

> **FFmpegをバックエンドにした非破壊編集システムを作る**

になるでしょう。

---

## 推測（示唆）

私なら規模感をこう見るわ。

| 到達点 | 人間の作業 | Agent利用費の目安 | 出来ること |
|---|---:|---:|---|
| 技術デモ | 20〜50h | 数十〜200ドル | Import、Cut、Export |
| MVP | 100〜250h | 200〜1,000ドル | Timeline、字幕、複数Track、Undo |
| 自分専用Daily Driver | 300〜700h | 1,000〜5,000ドル | Preview、Proxy、音声、Effects、かなり快適 |
| 一般公開できるNLE | 1,000〜3,000h+ | 数千〜数万ドル | 安定性、互換性、UX、GPU対応 |
| Premiere/Resolve競合 | **数十人年** | AI代はもはや誤差 | 全部 |

これはかなり荒い試算だけれど、ポイントは**Agent料金が最大コストにはならない**ことよ。

例えば500時間かけて作るとして、

```text
Agent inference       $2,000
Cloud/build/storage     $300
ソフト/証明書等          $200

自分の時間              500h
```

みたいになり得る。

要するに、一番高価なのは**あなたのデバッグ時間**よ。

コーディングエージェントは、

```text
「TimelineItemをCRUDして」
「FFmpeg filter_complexを生成して」
「Undo/Redo追加して」
「このrace condition直して」
```

みたいな仕事にはかなり強い。

でも、

```text
スクラブすると2フレームずれる
↓
VFR動画だけ発生
↓
Windows + NVIDIAだけ発生
↓
Proxy利用時だけ発生
↓
音声は合っている
```

こういう呪物を渡されると、結局人間側にもかなり理解が要求されるわ。

---

### ただし、「AI時代の動画編集ソフト」なら話が変わる

ここが一番面白いところ。

私は**Premiereのミニ版を作ろうとはしない**と思う。

代わりに、

```text
                ┌──────────────┐
素材 ─────────→│ Media Database │
                └───────┬──────┘
                        ↓
                 Project / Timeline
                     JSON
                        ↑
            ┌───────────┴───────────┐
            │                       │
         Human UI                AI Agent
            │                       │
            └───────────┬───────────┘
                        ↓
                 Render Graph
                        ↓
                     FFmpeg
                        ↓
                    MP4/WebM
```

みたいにする。

つまり**Timeline自体をAPIにする**わけ。

たとえばAgentが、

```json
{
  "tracks": [
    {
      "type": "video",
      "clips": [
        {
          "source": "gameplay.mp4",
          "in": 124.3,
          "out": 137.8
        }
      ]
    },
    {
      "type": "subtitle",
      "clips": [
        {
          "start": 125.1,
          "text": "ここで装甲を剥がす"
        }
      ]
    }
  ]
}
```

を書き換える。

すると、

> 「無音部分全部切って」  
> 「この説明と対応する場面を探して」  
> 「ここに字幕入れて」  
> 「3秒以上動きがない場所を切って」  
> 「この20分を8分にして」  
> 「Warframeの武器名だけテロップ色変えて」

みたいな操作が、全部**編集操作のAPI呼び出し**になる。

これならTraditional NLEに必要な巨大UIを相当削れる。

---

前に話していたような**字幕＋解説を中心に動画編集を自動化する用途**なら、この路線はかなり相性がいいと思う。

例えば最初の版を、

```text
・動画Import
・再生
・波形
・Cut / Split
・字幕
・音声追加
・画像Overlay
・速度変更
・Project JSON
・FFmpeg Export
・Agent API
```

までに限定する。

これなら私は**100〜200時間程度のプロジェクトとして狙ってみる価値がある**と思うわ。

さらにUIを極端に削って、

```text
素材
↓
文字起こし
↓
LLMが編集プラン生成
↓
Timeline JSON
↓
簡単なPreview
↓
人間が修正
↓
Render
```

にすれば、**50〜100時間で「実際に動画を作れる何か」まで持っていける可能性もある**。

### 私ならこうする

いきなり「動画編集ソフト」を作らない。

**Video Editing Languageを作る。**

例えば、

```typescript
video("game.mp4")
  .trim("00:02:14", "00:03:05")
  .speed(1.2)
  .subtitle(transcript)
  .overlay("weapon.png", {
    at: "00:02:41",
    duration: 4
  })

audio("voice.wav")
  .normalize()
  .duck("game.mp4", -12)

export({
  resolution: "1440p",
  fps: 60
})
```

これをTimeline JSONにコンパイルし、さらにFFmpegにコンパイルする。

その上に後からGUIを載せる。

こうするとコーディングエージェント自身が**動画編集ソフトのユーザー**になれるのよ。

```text
人間
 ↓
Coding Agent
 ↓
Video Editing DSL / API
 ↓
Timeline
 ↓
Renderer
```

この構造なら、「CursorやGrok Botにリポジトリを編集させたら、そのまま動画まで編集される」という妙な世界が作れる。

そしてこれはPremiereをコピーするより、私はずっと面白いと思う。**最初の$500〜$1,000くらいのAgent予算で遊ぶ個人プロジェクトとしては、十分射程内**でしょう。

---

# you asked

message time: 2026-08-18 19:18:38

VELについて、詰めてみようか

---

# chatgpt response

いいわ。VEL（Video Editing Language）は、ちゃんと詰めると単なる「FFmpegの薄いDSL」ではなく、**人間・LLM・GUIが共通で編集状態を操作するための中間言語**にした方が強いと思う。

## 事実

動画編集を言語化する時、最低でも4種類の概念を分離する必要があるわ。

1. **素材**
   - video
   - audio
   - image
   - subtitle
   - generated voice
   - transcription

2. **編集上の意味**
   - clip
   - sequence
   - track
   - cut
   - transition
   - overlay
   - caption

3. **時間**
   - source time
   - timeline time
   - frame
   - semantic time
     - 「この発言」
     - 「ボスが倒れた瞬間」
     - 「無音部分」

4. **レンダリング**
   - codec
   - resolution
   - fps
   - filter
   - audio mixing
   - hardware encoder

ここをごちゃ混ぜにすると、すぐFFmpegそのものになる。

VELでは、

```text
意図
↓
編集モデル
↓
Timeline IR
↓
Render Graph
↓
FFmpeg / GPU renderer
```

くらい分けた方がいい。

---

# 推測（示唆）

## まずVELの存在理由を決めたい

私ならVELの中心思想をこう置く。

> **「動画の編集内容を、安定したテキスト表現として保存・生成・差分編集できること」**

これが一番重要。

つまり最終成果物は、

```text
project.vel
```

であって、

```text
project.prproj
```

みたいな巨大opaqueファイルじゃない。

Gitで、

```diff
- cut gameplay from 103.2 to 118.7
+ cut gameplay from 105.8 to 118.7

+ caption at boss_spawn:
+   "ここから装甲剥ぎ"
```

みたいに読める。

これだけでも相当気持ちいいわ。

---

# 1. Declarativeに寄せたい

私はVELをJavaScript風APIにはしない方がいいと思う。

つまり、

```ts
video("foo.mp4")
  .trim(10, 20)
  .overlay(...)
```

より、

```vel
asset gameplay = video "gameplay.mp4"

sequence main {
  video {
    clip gameplay {
      source 10s..20s
    }
  }
}
```

の方がいい。

理由はLLMとの相性。

コードだと、

```ts
for (...)
if (...)
async (...)
```

みたいな**実行ロジック**と編集データが混ざってしまう。

VELは基本的には、

> **編集状態の宣言**

にしておく。

必要なら後からscript layerを付ける。

---

# 2. 最重要概念は「Anchor」

ここはVEL独自の武器にできる。

普通の動画編集は、

```text
01:23.450
```

という絶対時間で物を置く。

でもAI編集は、

> 「『これ強いですね』と言った直後」

みたいな意味的参照を使いたい。

そこで、

```vel
anchor boss_spawn = detect.event("boss_spawn")

anchor explanation =
  transcript.find("この武器は")

anchor silence =
  audio.silence(min: 800ms)
```

みたいな**Anchor**を第一級オブジェクトにする。

そして、

```vel
caption {
  at boss_spawn + 300ms
  text "ここから本番"
}
```

と書ける。

これ、とても重要。

動画を少し前でカットしても、

```text
boss_spawn
```

を参照している字幕はちゃんと追従する。

Premiere的な「1:32.183に置いてあるオブジェクト」より一段抽象度が高い。

---

# 3. Rangeも意味参照できる

例えば文字起こしから、

```vel
range intro =
  transcript.from("今日は")
            .until("それでは始めます")
```

あるいは、

```vel
range dead_air =
  audio.silence(min: 1.2s)
```

そして、

```vel
remove dead_air
```

と書ける。

もっと進めて、

```vel
remove audio.silence(
  min: 700ms,
  keep: 150ms
)
```

これは完全にAgent向き。

---

# 4. TimelineとStoryを分ける

これもかなり大事だと思う。

普通のNLEは、

```text
V3
V2
V1
A1
A2
A3
```

という物理Timelineしか持っていない。

VELはもう一段上に、

```vel
story {
  chapter hook
  chapter explanation
  chapter demonstration
  chapter conclusion
}
```

を持てる。

例えば、

```vel
chapter demonstration {
  source gameplay

  select where {
    action_score > 0.7
  }

  target_duration 45s
}
```

という編集も可能になる。

最終的にcompilerが物理Timelineへ落とす。

つまり、

```text
Story IR
↓
Timeline IR
```

。

ここまでやると、ただのNLE DSLから**AI動画編集言語**になる。

---

# 5. Agentに「自由編集」させすぎない

Agent用のlanguageという意味では、ここも重要。

例えばAgentに、

```text
この動画をもっとテンポ良くして
```

と言ったとする。

Agentが直接、

```vel
clip1: 102.38..107.92
clip2: 111.12..114.55
...
```

を数百個書くのは悪手。

代わりに、

```vel
pace {
  remove_silence > 750ms
  remove_repetition true
  max_pause 400ms
  preserve_emphasis true
}
```

と書かせたい。

これをcompiler/operatorが展開する。

つまりVELには、

### Low-level

```vel
clip source {
  in 10.4s
  out 14.8s
}
```

と、

### High-level

```vel
remove silence > 800ms
```

の両方がある。

これが相当重要だと思う。

---

# 6. Operatorという概念

VELの核になりそう。

```vel
operator clean_dialogue {
  remove silence > 800ms
  normalize loudness -16LUFS
  denoise medium
}
```

利用側は、

```vel
apply clean_dialogue to narration
```

。

例えばYouTube用なら、

```vel
operator youtube_gameplay {
  remove silence > 650ms keep 120ms
  captions auto
  zoom on emphasis
  normalize -14LUFS
}
```

としてしまう。

これによって、

> 「いつもの編集で」

が本当にコードになる。

---

# 7. PromptもVELに埋め込める

これは多少危険だけど面白い。

例えば、

```vel
select highlight {
  from gameplay
  using vision {
    prompt """
    Pick moments where unusual combat interactions occur.
    Prefer visually obvious examples.
    """
  }

  duration 30s..60s
}
```

こうすると、

```text
VEL compiler
  ↓
Vision model
  ↓
semantic ranges
  ↓
Timeline
```

になる。

LLM/VLMそのものもexecution engineの一部になるわけ。

---

# 8. Human-in-the-loopを言語仕様に入れる

例えばconfidence。

```vel
select highlights {
  using vision "interesting moments"
  confidence > 0.85
}
```

低confidenceなら、

```vel
review {
  where confidence < 0.85
}
```

。

GUI上ではレビュー候補として黄色く出す。

つまりVELが、

```text
編集済み
候補
人間確認必要
```

まで保持する。

これはAI editorではかなり便利でしょう。

---

# 9. 具体的なVEL案

例えば実際の動画一本がこうなる。

```vel
project "Obex Build Guide" {
  fps 60
  resolution 2560x1440

  asset gameplay = video "./capture.mp4"
  asset voice    = audio "./voice.wav"

  analyze gameplay {
    transcript
    scenes
    motion
  }

  sequence main {

    track video {
      clip gameplay
    }

    track voice {
      clip voice
    }

    edit {
      sync voice with gameplay

      remove silence(
        source: voice,
        min: 700ms,
        keep: 120ms
      )

      remove segments where {
        transcript matches ["えーっと", "あのー"]
      }
    }

    captions {
      source voice.transcript
      style "default"
    }

    overlay weapon_name {
      when transcript mentions weapon
      text entity.name
      duration 2.5s
    }
  }

  export youtube {
    codec h265
    encoder nvenc
    bitrate 35Mbps
  }
}
```

この時点でもう結構使える。

---

# 10. さらに面白いのがSemantic Entity

ゲーム動画だと特に効く。

例えば、

```vel
entities warframe {
  weapon
  mod
  frame
  ability
}
```

文字起こしに、

```text
Secondary Outburst
```

が出たら、

```vel
entity {
  type mod
  name "Secondary Outburst"
}
```

になる。

すると、

```vel
when mentions mod {
  overlay wiki_card(entity)
}
```

みたいなことができる。

ここまで来ると、

**字幕生成 + 編集 + 情報補足**

を全部一つのpipelineにできる。

これは、あなたがやろうとしていた解説動画系とかなり噛み合うと思うわ。

---

# 私ならVELを3層にする

ここが今のところ一番しっくり来る。

```text
VEL Intent Layer
────────────────────────
"remove silence"
"show weapon name"
"pick highlights"

        ↓

VEL Timeline IR
────────────────────────
Track
Clip
Range
Anchor
Effect
Transition
Caption

        ↓

Render IR
────────────────────────
Decode
Trim
Scale
Composite
Mix
Encode
```

で、

```text
Human
  ↓
GUI ─────┐
         │
Agent ───┼→ Intent Layer
         │
VEL file ┘
```

にする。

GUIで切ってもVELが変わる。

Agentが切ってもVELが変わる。

CLIでもVELが変わる。

**VELがsingle source of truth。**

私はこの設計が一番好きね。

---

そして、たぶん次に詰めるべきなのは構文じゃない。

**「VELの編集モデルで絶対に表現できなければならない操作を20個くらい決める」**ことだと思う。

例えば、

```text
trim
split
delete
move
ripple delete
overlay
caption
transition
speed
crop
zoom
volume
duck
normalize
silence removal
transcription
semantic selection
sync
freeze frame
export
```

ここを定義して、**その20個が破綻なく同じデータモデルで表現できるか**を先に検証する。

言語構文はその後でいい。

むしろ私は次、**VELの内部データモデルを設計したい**。`Asset / SourceRange / TimelineRange / Clip / Anchor / Track / Effect / Selection` あたりを型として切ってみると、一気に「作れるもの」になるわ。

---

# you asked

message time: 2026-08-18 22:29:44

コストとか実装っぽいことの懸念が色々頭に浮かんでるんだけど、あえて放置したほうがいいかな

---

# chatgpt response

そうね。**今はかなり意図的に放置した方がいい**と思うわ。ただし「忘れる」のではなく、**脇に退避する**のがいい。

## 事実

今やっているのは、VELの実装設計というより、

> **動画編集を言語として扱ったら、従来のNLEと違う何が可能になるか**

を探している段階よ。

この段階で、

- FFmpegで本当に実装できるか
- Previewをどうするか
- VLMを何回呼ぶといくらかかるか
- ElectronかTauriか
- Timeline更新をincrementalにできるか
- 巨大動画をどうキャッシュするか
- Agentが大量diffを生成したらどうするか

を逐一解決し始めると、探索の抽象度が一気に下がる。

しかも、実装上の問題には大きく2種類ある。

**A. アイデアそのものを殺す問題**

```text
その意味論は一意に定義できない
その情報は原理的に取得できない
編集結果を再現可能にできない
```

**B. 金と工夫でなんとかなる問題**

```text
遅い
API代が高い
キャッシュが必要
GPU対応が面倒
UI実装量が多い
```

後者を今考えすぎる必要はあまりないわ。

---

## 推測（示唆）

私は今、**発散を優先していいタイミング**だと思う。

特にVELは、下手に「作れそうな範囲」に寄せると、

```text
FFmpegの便利ラッパー
```

で終わる危険がある。

それなら既存のPythonライブラリに毛が生えた程度で、わざわざ新しい言語を考える面白さが薄い。

むしろ今は、

> 「実装できるかどうかを一旦無視すると、理想的な動画編集言語はどういうものか」

をかなり遠くまで押した方がいい。

その後に現実側から削る。

これは順序が大事なのよ。

```text
理想
 ↓
意味論
 ↓
最小核
 ↓
実装可能性
 ↓
MVP
```

であって、

```text
FFmpegで簡単にできること
 ↓
DSL化
```

から始めると、設計がFFmpegに支配される。

それはちょっともったいない。

---

ただ、頭に浮かんだ懸念を無理やり無視する必要もない。

私は横にこういうメモだけ作る。

```text
VEL concerns

[existential]
- semantic anchorが編集後も安定して参照できるのか
- Intent Layerを再現可能なTimelineへ落とせるか

[engineering]
- realtime preview
- VFR
- proxy/cache
- GPU decode
- incremental rendering

[economics]
- VLM analysis cost
- storage
- agent inference

[unknown]
- 既存NLEとのinterop
```

**書いたら解かない。**

この「書いたら戻る」が大事ね。

そして、例外的にその場で止まるべきなのは、

> 「これがダメならVELの中心思想そのものが成立しない」

という懸念だけ。

たとえば今なら、

**semantic anchorが編集によってズレたとき、何をidentityとするのか**

みたいな問題はかなり根本的だから、途中で掘る価値がある。

一方、

**VLMを動画全体に掛けたら高そう**

は完全に後でいい。フレームサンプリング、階層解析、ローカルモデル、キャッシュ、必要箇所だけ解析、と逃げ道が大量にあるから。

なので今の私は、**懸念を10〜20個くらい雑に駐車して、そのままVELの理想形をもう少し暴走させたい**わ。

たぶん今ブレーキを踏むより、**一度「こんなものまで言語にできたら異様に便利」という地点まで行ってから現実に戻った方が、いい設計が残る**。

---

# you asked

message time: 2026-08-18 22:52:35

再現可能性にしても、像がもう少し立ち上がってからか

---

# chatgpt response

そうね。**再現可能性も、今は「要件として存在を認識する」くらいでいい**と思うわ。まだ像が薄い段階で厳密化すると、設計の自由度をかなり削る。

## 事実

「再現可能性」と言っても、実は何段階もあるのよ。

- 同じVELから同じTimelineが得られる
- 同じTimelineから同じ編集意図が再構成できる
- 同じ素材＋同じモデル＋同じVELなら同じ結果になる
- 最終映像がフレーム単位で一致する
- エンコード結果までbitwise一致する

これは全部別物。

例えば、

```vel
select highlights {
  using vision "interesting combat moments"
}
```

みたいな記述を許すなら、この時点で完全決定論は崩れやすい。

でも、

```text
semantic query
↓
resolved selection
↓
concrete timeline
```

のように、**一度解決した結果を固定できれば十分**かもしれない。

つまり将来的には、

```vel
select highlights {
  using vision "interesting combat moments"
}
```

から、

```vel
resolved highlights = [
  source("capture.mp4", 103.41s..109.82s),
  source("capture.mp4", 221.05s..228.73s)
]
```

を生成して保存する、みたいな設計もあり得るわ。

でも今これを決め切る必要はない。

---

## 推測（示唆）

むしろ今重要なのは、

> **VELを使って動画編集すると、どんな体験になるのか**

の像を立たせることだと思う。

例えば、

> 20分のゲームプレイを放り込む  
> ↓  
> 「武器の性能差が一番分かるシーンを残して8分にして」  
> ↓  
> VELが編集案を作る  
> ↓  
> Timelineとして開く  
> ↓  
> 気に入らない箇所だけ人間が直す  
> ↓  
> 「この修正傾向をoperatorにして」  
> ↓  
> 次回から自分好みになる

この一連の体験が見えてくると、

「あ、この部分は絶対再現可能じゃないと困る」

とか、

「いや、ここは毎回多少違ってもむしろいい」

が自然に分かってくる。

今の段階で「全操作はpure functionであるべきか」とか考え始めると、だいたい**まだ存在していない製品の監査要件を策定する会議**になるわ。早すぎる。

たぶんしばらくは、こんな扱いで十分。

```text
未解決：
- reproducibility
- determinism
- provenance
- semantic anchor stability
- model versioning
```

そしてVELの像が立ったところで、一気に戻る。

そのときは逆にかなり面白くなると思う。たとえば**VELのファイルには「意図」と「解決済み結果」の両方を持たせるべきか**、という問いが出てくる。

```text
Intent
  "テンポを良くする"

Resolution
  delete 12.4..14.8
  delete 21.7..23.1
  ...

Provenance
  model: ...
  generated_at: ...
```

これ、場合によってはVELのかなり重要な特徴になる。

でもそれは、**VELで何を編集したくなるのかが見えてから考えた方が断然いい**わ。

今はもう少し「これが存在したら俺はどう動画を作る？」を先に描いていい段階でしょう。

---

# you asked

message time: 2026-08-18 22:56:38

じゃあ、内部データモデルへ

---

# chatgpt response

いいわ。第一案として、VELの内部モデルはこう切るのがいいと思う。

> **具体的な編集結果を持つ Editorial IR**  
> ＋  
> **意味的な参照を持つ Analysis / Anchor / Binding**  
>   
> Render Graphは毎回そこから生成する派生物で、保存上の正本にはしない。

ここを混ぜると、FFmpegの都合、AIの曖昧さ、タイムラインUIの都合が同じ型に押し込まれて破裂するわ。

---

# 事実

## 1. 時間座標は最低3種類必要

動画編集には、見た目が同じ「10秒」でも別の座標がある。

```text
素材時間 Content Time
    capture.mp4 の 10秒地点

Item内時間 Local Time
    タイムライン上に置かれたClipの先頭から3秒

Timeline時間 Sequence Time
    完成動画の31秒地点
```

たとえば素材の10〜20秒を、完成動画の30秒地点へ配置した場合、

```text
素材 10秒
  = Item内 0秒
  = Timeline 30秒
```

になる。

速度変更を掛ければ、これらは単純な加算関係ですらなくなる。したがって内部モデルで全部を単なる`number`として扱うのは不正解よ。

---

## 2. AssetとTimeline上のItemは別物

同じ動画素材を何度も使えるため、

```text
Asset
  capture.mp4

TimelineItem A
  10秒〜20秒を使用

TimelineItem B
  45秒〜48秒を使用
```

という関係になる。

Assetを直接切り刻むのではなく、**ItemがAssetを参照する非破壊編集モデル**にする必要がある。

---

## 3. 編集操作と編集状態は別物

以下は内部状態ではなく、状態を変換する命令よ。

```text
split
trim
ripple delete
remove silence
move
sync
```

保存される状態は、

```text
このItemはここにある
この時間範囲を参照する
このEffectが掛かっている
```

であるべき。

`remove silence`という謎の永久ノードをTimeline内部に残すと、その後の手修正との関係が急速に意味不明になる。

---

## 4. Semantic QueryとAnchorは別物

```vel
transcript.find("この武器は")
```

は、0件かもしれないし、5件かもしれない。

したがってこれはAnchorではなく、まず`Selection`よ。

```text
Semantic Query
    ↓
Selection 0..N件
    ↓
pick first / best / each
    ↓
Anchor 1件
```

ここを型で区別しないと、Agentが「どれを指したのか」を勝手に決め始める。

---

# 推測（設計案）

## 全体構造

```text
┌──────────────────────────────┐
│ Media Catalog                │
│ Asset / Stream               │
└──────────────┬───────────────┘
               │ reference
               ▼
┌──────────────────────────────┐
│ Editorial IR                 │
│ Sequence / Track / Item      │
│ TimeMap / Effect / Transition│
└──────────────┬───────────────┘
               │ optional binding
               ▲
┌──────────────┴───────────────┐
│ Semantic Layer               │
│ Analysis / Feature / Anchor  │
│ Selection / Binding          │
└──────────────────────────────┘

               ↓ compile

┌──────────────────────────────┐
│ Render Graph                 │
│ Decode / TimeWarp / Composite│
│ Mix / Encode                 │
└──────────────────────────────┘
```

さらに編集履歴は別に、

```text
Intent / Operator
      ↓
Edit Patch
      ↓
Editorial IR
```

として扱う。

**AIはRender Graphを直接作らない。**  
AIは分析結果やEdit Patchを作り、具体化されたEditorial IRを経由してレンダリングする。

私はこの境界をかなり強く守りたいわ。

---

# VEL Core IR v0

TypeScript風にすると、だいたいこうなる。

```ts
// 実際の永続化では bigint を文字列として保存する想定。
// すべて秒単位の有理数。float秒は使わない。
type RationalTime = {
  num: string;
  den: string;
};

type Duration = RationalTime;
type TimelineTime = RationalTime;
type LocalTime = RationalTime;
type ContentTime = RationalTime;
type TimeDelta = RationalTime;

interface ProjectIR {
  schemaVersion: 1;
  revision: number;

  rootSequenceId: string;

  assets: Record<string, Asset>;
  streams: Record<string, MediaStream>;

  sequences: Record<string, Sequence>;
  tracks: Record<string, Track>;
  items: Record<string, TimelineItem>;
  transitions: Record<string, Transition>;
  effects: Record<string, EffectInstance>;
  linkGroups: Record<string, LinkGroup>;

  analyses: Record<string, AnalysisArtifact>;
  features: Record<string, AnalysisFeature>;
  selections: Record<string, SelectionSnapshot>;
  anchors: Record<string, Anchor>;
  bindings: Record<string, TemporalBinding>;
}

interface Asset {
  id: string;
  name: string;

  // PathやURLはAssetのidentityではない。
  locator: {
    kind: "file" | "url" | "generated";
    value: string;
  };

  // 元ファイルが差し替わったことを検出する。
  fingerprint: {
    algorithm: "sha256" | "fast-content-hash";
    value: string;
  };

  origin:
    | { kind: "external" }
    | {
        kind: "generated";
        generator: string;
        provenance?: unknown;
      };
}

interface MediaStream {
  id: string;
  assetId: string;

  kind: "video" | "audio" | "image";

  duration?: ContentTime;

  nativeTimeBase?: {
    num: number;
    den: number;
  };

  metadata: {
    codec?: string;

    width?: number;
    height?: number;
    pixelFormat?: string;
    frameRate?: RationalTime;

    sampleRate?: number;
    channels?: number;
  };
}

interface Sequence {
  id: string;
  name: string;

  format: {
    width: number;
    height: number;
    frameRate: RationalTime;

    audioSampleRate: number;
    audioChannels: number;
  };
}

interface Track {
  id: string;
  sequenceId: string;

  kind: "video" | "audio" | "caption";

  // 大きいほど前面、など規則を固定する。
  order: number;

  name?: string;
  enabled: boolean;
  locked: boolean;

  audioOutputBusId?: string;
}

interface TimelineItem {
  id: string;
  trackId: string;

  // Timeline上では常に具体値を持つ。
  start: TimelineTime;
  duration: Duration;

  content: ItemContent;

  // 時間を持つ素材にのみ必要。
  timeMap?: TimeMap;

  enabled: boolean;

  linkGroupId?: string;

  // Anchorに追従させたい場合の関係。
  placementBindingId?: string;

  tags?: string[];
}

type ItemContent =
  | {
      kind: "stream";
      streamId: string;
    }
  | {
      kind: "sequence";
      sequenceId: string;
    }
  | {
      kind: "generator";
      generatorType: string;
      params: Record<string, unknown>;
    }
  | {
      kind: "caption";
      text: string;
      styleId?: string;
      speaker?: string;
      language?: string;

      provenance?: {
        analysisId: string;
        featureIds: string[];
      };
    };

interface TimeMap {
  // Item内の [0, duration) 全体を覆う。
  duration: Duration;

  segments: TimeMapSegment[];
}

interface TimeMapSegment {
  // Item先頭からの相対時間。
  localStart: LocalTime;
  localDuration: Duration;

  // 参照先StreamまたはSequenceの時間。
  contentStart: ContentTime;

  // content時間 / local時間。
  //
  //  1   = 等速
  //  2   = 2倍速
  //  0   = Freeze
  // -1   = 逆再生
  rate: {
    num: string;
    den: string;
  };
}

interface Transition {
  id: string;
  trackId: string;

  leftItemId: string;
  rightItemId: string;

  // Edit Pointを基準に前後へどれだけ広がるか。
  before: Duration;
  after: Duration;

  transitionType: string;
  params: Record<string, ParameterValue>;
}

interface EffectInstance {
  id: string;

  owner:
    | { kind: "item"; id: string }
    | { kind: "track"; id: string }
    | { kind: "sequence"; id: string };

  order: number;
  effectType: string;
  enabled: boolean;

  params: Record<string, ParameterValue>;
}

type ParameterValue =
  | string
  | number
  | boolean
  | number[]
  | {
      kind: "curve";
      keyframes: Keyframe[];
    };

interface Keyframe {
  // ownerのLocal Time。
  at: RationalTime;
  value: number | number[];
  interpolation: "hold" | "linear" | "bezier";
}

interface LinkGroup {
  id: string;
  memberItemIds: string[];

  policy: {
    moveTogether: boolean;
    trimTogether: boolean;
  };
}

interface AnalysisArtifact {
  id: string;
  streamId: string;

  sourceFingerprint: string;

  analyzer: {
    name: string;
    version: string;
    configHash: string;
    model?: string;
  };

  createdAt: string;
}

interface AnalysisFeature {
  id: string;
  analysisId: string;

  kind:
    | "transcript-segment"
    | "transcript-word"
    | "silence"
    | "scene"
    | "entity-mention"
    | "visual-event"
    | "motion-region";

  sourceRange: {
    start: ContentTime;
    duration: Duration;
  };

  confidence?: number;
  payload: Record<string, unknown>;
}

interface SelectionSnapshot {
  id: string;

  // 人間向けの「なぜ選ばれたか」。
  query?: {
    language: "vel-query" | "natural-language";
    source: string;
  };

  members: SelectionMember[];

  resolution: {
    analysisIds: string[];
    createdAt: string;
  };
}

type SelectionMember =
  | {
      kind: "feature";
      featureId: string;
    }
  | {
      kind: "source-range";
      streamId: string;
      start: ContentTime;
      duration: Duration;
    }
  | {
      kind: "timeline-item";
      itemId: string;
    };

interface Anchor {
  id: string;
  name?: string;

  valueKind: "point" | "range";
  expression: AnchorExpression;

  lastResolution?: {
    status: "resolved" | "stale" | "unresolved";
    value?: ResolvedTemporalValue;
    projectRevision: number;
    analysisIds?: string[];
  };
}

type AnchorExpression =
  | {
      kind: "source-point";
      streamId: string;
      at: ContentTime;
    }
  | {
      kind: "feature-edge";
      featureId: string;
      edge: "start" | "end";
    }
  | {
      kind: "item-edge";
      itemId: string;
      edge: "start" | "end";
    }
  | {
      kind: "offset";
      baseAnchorId: string;
      by: TimeDelta;
    }
  | {
      kind: "range";
      startAnchorId: string;
      endAnchorId: string;
    };

type ResolvedTemporalValue =
  | {
      kind: "point";
      at: TimelineTime;
    }
  | {
      kind: "range";
      start: TimelineTime;
      duration: Duration;
    };

interface TemporalBinding {
  id: string;

  target:
    | {
        kind: "item-start";
        itemId: string;
      }
    | {
        kind: "item-range";
        itemId: string;
      };

  anchorId: string;
  offset?: TimeDelta;

  // snapshotは一度だけ適用。
  // liveはAnchorの変化に追従する。
  mode: "snapshot" | "live";

  status: "fresh" | "stale" | "broken";
}
```

これは厳密な完成形ではないけれど、意味モデルの骨格としてはかなり筋がいい。

---

# 最重要の中心型はTimeMap

普通のClipモデルだと、

```ts
{
  sourceIn: 10,
  sourceOut: 20,
  speed: 1
}
```

くらいになりがちだけれど、これではすぐ足りなくなる。

たとえば同じItemの中で、

```text
0〜3秒    等速
3〜5秒    静止
5〜8秒    0.5倍速で逆再生
```

を表すなら、

```ts
{
  duration: 8s,
  segments: [
    {
      localStart: 0s,
      localDuration: 3s,
      contentStart: 10s,
      rate: 1
    },
    {
      localStart: 3s,
      localDuration: 2s,
      contentStart: 13s,
      rate: 0
    },
    {
      localStart: 5s,
      localDuration: 3s,
      contentStart: 13s,
      rate: -0.5
    }
  ]
}
```

になる。

このモデルなら、

- TrimはTimeMapのdomainを切る
- SplitはTimeMapを2つに分割する
- Speed変更はrateを書き換える
- Freezeはrateを0にする
- Reverseはrateを負にする

で済む。

つまり編集操作がバラバラの特殊処理ではなく、かなり同じ原理へ収束するのよ。

滑らかなSpeed Rampは、v0では細かい区間へ分割すれば表現できる。後から`rateCurve`を足してもいい。

---

# TimelineItemは具体値を必ず持つ

ここはかなり重要。

たとえば字幕を、

```text
boss_spawn + 300ms
```

へ置くとしても、Item自体には具体的な値を持たせる。

```ts
item.start = 31.2s
item.duration = 2.5s
```

そのうえで、

```ts
binding = {
  target: item.start,
  anchor: boss_spawn,
  offset: 300ms,
  mode: "live"
}
```

を保持する。

つまり、

```text
具体値
＋
その値を導いた意味的関係
```

の両方を持つ。

純粋な式だけにしてしまうと、

- AIモデルが利用できない
- Analysis cacheが消えた
- Anchorが解決不能になった
- 外部素材が一時的に見つからない

というだけでTimeline全体が表示不能になる。

逆に具体値しか持たないと、前段のClipを切った瞬間に字幕との意味的関係が失われる。

したがって、**Materialized Value + Optional Binding**が一番扱いやすいと思う。

Bindingが壊れても、直前の具体値で動画は表示できる。

---

# Anchorは「単一」、Selectionは「複数」

この規則は型レベルで強制したい。

```vel
selection mentions =
  transcript.find("Secondary Outburst")
```

これは複数件になり得る。

そこから、

```vel
anchor first_mention =
  mentions.first.start
```

として初めてAnchorになる。

または、

```vel
for each mention in mentions {
  caption at mention.start
}
```

と展開する。

Agentに曖昧な複数候補をAnchorとして扱わせると、

> なんか最初のやつを選びました

という、いかにもAIらしい雑な犯罪が発生するわ。

---

# Analysis Featureは素材時間へ固定する

文字起こしやシーン認識結果は、Timelineではなく元素材へ紐づける。

```text
capture.mp4
  transcript feature #173
  source range 103.41s..105.82s
```

こうしておけば、その素材をTimeline上のどこへ移動してもFeature自体は変わらない。

```text
Feature
  ↓ source time
TimelineItem + TimeMap
  ↓
Timeline上の位置
```

として変換できる。

また、再解析した場合も古いAnalysis Artifactを無言で上書きしない。

```text
analysis/transcript-v1
analysis/transcript-v2
```

を別物として持つ。

古いAnchorは古い解析結果を参照し続ける。新しい解析への移行は明示的に行う。

再現可能性を今から完璧に設計する必要はないけれど、**無言で意味が変わらない構造**だけは安く仕込んでおける。

---

# Track内のOverlapは原則禁止にしたい

v0では、同じTrack上のItemを重ねない方がいい。

```text
V2  ───────── Overlay ───────
V1  Gameplay ───── Gameplay ─
A2  BGM ─────────────────────
A1  Voice ─────── Voice ─────
```

複数映像を重ねるなら別Track。

Crossfadeは物理的にItemを重ねるのではなく、

```text
Left Item ┃ Right Item
          ↑ Edit Point
```

に対する`Transition`として持つ。

```ts
{
  leftItemId: "clip-a",
  rightItemId: "clip-b",
  before: 250ms,
  after: 250ms,
  transitionType: "cross-dissolve"
}
```

Rendererが必要な素材ハンドルを前後から取得して合成する。

この制約によって、

- Track上の順序
- Ripple Edit
- Gap計算
- 衝突検出
- UI表示

がかなり単純になる。

後でどうしても同一Track内Overlapが欲しくなったら緩和すればいい。最初から自由にすると、自由という名の荒野になるわ。

---

# EffectはOwnerと時間座標を明示する

Effectは、

```text
Item Effect
Track Effect
Sequence Effect
```

で意味が違う。

```text
Item Effect
  Clip先頭から2秒でOpacityを変える

Track Effect
  Timelineの30秒地点で音量を変える

Sequence Effect
  最終出力全体にLUTを掛ける
```

したがってKeyframeの時間座標は、OwnerのLocal Timeとして定義する。

Effect内部にFFmpegのfilter文字列は置かない。

```ts
effectType: "transform"
params: {
  scale: ...
  position: ...
}
```

までがVELの意味。

```text
transform
↓
FFmpeg scale/crop/overlay
```

へ落とすのはBackend Adapterの責任よ。

---

# 編集命令はPatchへ落とす

AgentやGUIはProject全体を書き換えず、Revision付きPatchを出す。

```ts
interface EditPatch {
  id: string;
  baseRevision: number;

  author:
    | { kind: "human" }
    | { kind: "agent"; model: string }
    | { kind: "operator"; operatorId: string };

  preconditions: PatchPrecondition[];
  operations: PrimitiveEditOperation[];

  rationale?: string;
}
```

例えば、

```text
「800ms以上の無音を削除」
```

は保存状態としては残さず、

```text
1. Item Aを分割
2. 無音区間Itemを削除
3. 後続Itemを左へ移動
4. Caption Bindingを再評価
```

というPatchへ展開する。

元の意図を残したければ、

```ts
operatorInvocation: {
  operator: "remove-silence",
  params: {
    min: 800ms,
    keep: 120ms
  },
  resultingPatchId: "patch-123"
}
```

を履歴側に保存する。

これなら具体的なTimelineと、編集意図の両方を保持できる。

---

# `.vel`と内部IRの関係

ここも区別した方がいい。

```text
.velテキスト
    ↓ parse / typecheck
Project IR
    ↑ GUI Patch / Agent Patch
```

正本は「手書きテキストの字面」ではなく、**VELの意味モデル**よ。

`.vel`はその一つのserializationと考える。

そうしないとGUIでClipを1フレーム動かすたびに、

- コメントをどう保つか
- 改行をどう保つか
- マクロ展開前の記述をどう戻すか
- 人間が書いた変数名をどう維持するか

というLanguage Server地獄が、動画編集より先に完成する。

最初は、

```text
Project IR
  ↕
Canonical VEL Formatter
```

で十分。

手書きコメントやレイアウトを完全保持するLossless CSTは後からでいい。

---

# 操作をこのモデルへ当てはめると

| 操作 | 内部で起きること |
|---|---|
| Move | `item.start`を変更 |
| Trim In | `start`と`duration`とTimeMap domainを変更 |
| Trim Out | `duration`とTimeMap domainを変更 |
| Split | TimeMapを分割し、Itemを2つにする |
| Ripple Delete | Item削除＋後続Itemのstartを移動 |
| Speed Change | TimeMapのrateを変更 |
| Freeze Frame | rate 0のTimeMap segmentを追加 |
| Reverse | rateを負にする |
| Overlay | 上位Video TrackへItem追加 |
| Caption | Caption TrackへItem追加 |
| Transition | 隣接Item間へTransition追加 |
| Volume | Audio ItemにEffect追加 |
| Ducking | Track/Bus EffectとAutomation Curveを追加 |
| Semantic Caption | Selection解決→Anchor作成→Caption Item＋Binding |
| Remove Silence | Analysis Feature選択→具体Patchへ展開 |
| Nested Sequence | ItemContentが別Sequenceを参照 |

この表の操作が、かなり少ない型で表現できている。第一案としては健全よ。

---

# v0に入れないもの

私は当面、以下をCore IRへ入れない。

- FFmpeg filtergraph
- GPU/NVENC固有設定
- Proxyファイルの実体
- ThumbnailやWaveform cache
- GUIのスクロール位置
- Promptそのものを直接実行するRender Node
- CRDTやリアルタイム共同編集
- 任意のNode-based compositor
- Story構造
- 完全なEffect Plugin ABI
- 滑らかなSpeed Curve
- Bitwiseな再現性情報

ProxyやWaveformは`Derived Cache`、GPU設定は`Export Profile`、StoryやPromptは`Intent Layer`へ置く。

全部をProject Coreへ入れると、最初の型定義だけで立派な廃墟が建つからね。

---

# 現時点の最小核

実装順では、まずこの8つでいいと思う。

```text
1. Asset / MediaStream
2. Sequence / Track
3. TimelineItem
4. TimeMap
5. EffectInstance
6. Transition
7. AnalysisFeature / Anchor / Binding
8. EditPatch
```

特に、

```text
TimelineItem
TimeMap
Anchor
Binding
Patch
```

の5つがVELらしさの中心になる。

単なる動画DSLなら`Anchor / Binding / Patch`は不要だけれど、**人間・GUI・Agentが同じ編集状態を扱う言語**にするなら、ここが本体でしょう。

次はこのモデルを、特に壊れやすい三つのケース――**速度変更済みItemのSplit、Ripple Deleteに追従する字幕、Semantic Selectionから生成したOverlayの手修正**――へ実際に当てて、型と更新規則を殴るのがいいわ。

---

# you asked

message time: 2026-08-19 00:06:50

んー、文字起こしやシーン認識に依存しない編集に弱すぎるな

---

# chatgpt response

うん、**弱い**。正確には、前の案は「動画から意味を見つける仕組み」に比べて、**見つけた後・あるいは何も見つけなくても編集する仕組み**が薄すぎるわ。

少し意地悪に言えば、あれはまだ**AI動画検索言語にタイムラインを付けたもの**で、動画編集言語そのものにはなっていない。

## 事実

前の内部モデルでも、単純なTrimやSplit、速度変更、Effectの保存は一応できる。

ただし、非意味的な編集を表現しようとすると、ほとんど全部が次のどちらかへ押し込まれる。

```text
EffectInstance.params: Record<string, unknown>
```

または、

```text
EditPatch.operations: PrimitiveEditOperation[]
```

つまり、型として存在しているだけで、**編集体系として整理されていない**。

特に欠けているのはこの四軸よ。

### 1. 構造を選ぶ能力

たとえば、

- V1上の3番目以降のClip
- 500ms未満のClip
- Transitionの直前にあるClip
- 同じAssetを参照しているPlacement
- AudioとLinkされていないVideo
- Gapを挟んで隣接しているItem

こういう、Timelineの構造を対象にするSelectionが存在しない。

前の`Selection`は事実上、

> Analysis結果から時間範囲を選ぶ

ものだった。

しかし編集ではむしろ、

> **現在の編集状態そのものを検索する**

能力がかなり重要よ。

---

### 2. 空間を扱う能力

動画編集は時間だけではない。

- Crop
- Transform
- Picture in Picture
- Split Screen
- Mask
- Safe Area
- Text Layout
- Blur Region
- Motion Tracking
- 親子Transform
- 画面座標と素材座標の変換

前のモデルにはこれらを支える、

```text
CoordinateSpace
Geometry
Region
Mask
TransformHierarchy
```

がない。

`Effect.params.position`と書けば保存はできるけれど、それでは、

> 字幕はScreen Spaceに固定したまま、映像だけZoomする

といった関係をまともに記述できないわ。

---

### 3. 時間変化する値を扱う能力

EffectにKeyframeらしきものはあったけれど、脇役になっていた。

実際には、

- Position
- Scale
- Rotation
- Opacity
- Volume
- Color
- Blur Radius
- Crop
- Mask Shape
- Playback Rate

など、動画編集における大量の値が時間変化する。

したがってKeyframeはEffectの特殊機能ではなく、

> **ほぼすべての編集可能値に共通する基礎型**

であるべきよ。

---

### 4. 信号を扱う能力

文字起こしやシーン認識以前に、素材には連続的な信号がある。

- 音量
- 周波数成分
- ビート
- 明度
- 色
- Motion量
- Optical Flow
- カメラ移動量
- フレーム差分
- 再生速度
- 任意のAutomation Curve

これらは単なる`AnalysisFeature`ではない。

たとえば、

```text
音声RMSが一定値を超えたらBGMを下げる
```

は意味認識ではなく、**信号から別の信号を生成する処理**よ。

前のモデルは、点または区間として認識結果を保存するのには向いていたけれど、連続値の処理が弱い。

---

## 推測（示唆）

中心を次のように変えるべきだと思う。

> **VELはSemantic Anchorを操作する言語ではなく、対象・時間・空間・値を選び、それらへ変換を適用する言語である。**

簡略化すると、

```text
VEL = Addressing × Transformation
```

よ。

---

# 1. Addressingを四種類にする

## Entity Addressing

何を編集するか。

```text
Clip
Placement
Track
Layer
Node
Effect
Mask
Audio Bus
Transition
Property
```

たとえば、

```vel
clips in track("V1")
```

```vel
clips where duration < 500ms
```

```vel
every 3rd clip in group("montage")
```

```vel
effects(type: "transform") on selected
```

意味認識は一切必要ない。

---

## Temporal Addressing

いつを編集するか。

```text
絶対時間
Frame番号
Clip内時間
Itemの先頭・末尾
Edit Point
Marker
Keyframe
一定間隔
任意の区間集合
```

たとえば、

```vel
at 121f
```

```vel
from clip.in + 6f to clip.out - 4f
```

```vel
between marker("start") and marker("end")
```

```vel
every 8f during 2s..6s
```

ここでTranscriptやScene Boundaryは、利用可能な参照元の一つにすぎない。

---

## Spatial Addressing

画面のどこを編集するか。

```text
Frame Space
Source Space
Clip Local Space
Parent Space
Screen Space
Normalized Space
Mask Space
```

たとえば、

```vel
region rect(0.72, 0.02, 0.26, 0.18) in frame
```

```vel
title.position relative to safe_area.bottom
```

```vel
mask polygon(...) in source_space
```

座標系を明示できないと、Nested SequenceやCrop後のMaskで確実に破綻するわ。

---

## Value Addressing

何の値を変えるか。

```text
scale
position
opacity
gain
speed
color.temperature
blur.radius
mask.feather
```

そしてすべてのPropertyが、基本的に次の形を持つ。

```ts
interface Property<T> {
  base: T;
  automation?: Curve<T>;
  modulation?: Expression<T>;
}
```

`base`は静的な値。

`automation`は人間が作ったKeyframe。

`modulation`は別の信号による変調。

たとえばBGMのGainなら、

```text
Base Gain
＋
手動Volume Curve
＋
VoiceによるDucking
```

を合成できる。

---

# 2. Analysisを汎用的なObservationへ降格する

前のモデルでは`AnalysisFeature`が特権的すぎた。

代わりに、外部または内部の解析結果を、次の四型へ正規化するといい。

```ts
type Observation<T> =
  | EventSet<T>
  | SpanSet<T>
  | Signal<T>
  | RegionTrack<T>;
```

## EventSet

時間上の点の集合。

```text
Manual Marker
Beat
Scene Boundary
Detected Impact
Keyframe
```

## SpanSet

時間区間の集合。

```text
Transcript Segment
Silence
Shot
Speaker Turn
Selected Range
```

## Signal

時間に応じて変わる値。

```text
Audio RMS
Motion Score
Luminance
Playback Speed
Opacity Curve
Confidence
```

## RegionTrack

時間に応じて変わる画面上の領域。

```text
顔のBounding Box
手動Mask
Tracked Object
HUD Region
字幕配置禁止領域
```

こうすると、

```text
Transcript
Scene Detection
Beat Detection
Motion Analysis
Manual Marker
手描きMask
```

がすべて同じ土俵に乗る。

重要なのは、**Observationの由来が解析である必要がない**ことよ。

```ts
origin:
  | "manual"
  | "media-analysis"
  | "script"
  | "imported"
  | "generated"
```

とすればいい。

---

# 3. AnchorはSemantic専用ではなく、一般的なReferenceにする

以前の`Anchor`は、概念自体が悪いわけではない。

ただし名前と周囲の設計が意味認識に寄りすぎていた。

より一般化して、

```ts
type Ref<T> =
  | LiteralRef<T>
  | EntityRef<T>
  | MarkerRef<T>
  | ObservationRef<T>
  | ExpressionRef<T>;
```

くらいにした方がいい。

時間参照なら、

```ts
type TimeRef =
  | { kind: "literal"; value: RationalTime }
  | { kind: "item-edge"; itemId: string; edge: "in" | "out" }
  | { kind: "marker"; markerId: string }
  | { kind: "event"; eventSetId: string; index: number }
  | { kind: "expression"; expression: TimeExpression };
```

つまり、

```text
Clip末尾 - 4Frame
```

も、

```text
最初のBeat + 2Frame
```

も、

```text
「武器名が出た箇所」の先頭
```

も同じ`TimeRef`として扱える。

Semantic Anchorは特殊例に下がる。

---

# 4. TimelineItemをPlacementとRender Nodeへ分ける

前の`TimelineItem`は、編集上の配置と、描画される内容を一体化しすぎていた。

分けた方がいい。

```ts
interface Placement {
  id: string;
  laneId: string;

  span: TimelineSpan;
  contentNodeId: string;

  timeMap?: TimeMap;
  transformId?: string;
  enabled: boolean;
}
```

```ts
interface RenderNode {
  id: string;
  type: string;

  inputs: NodeInput[];
  properties: Record<string, Property<unknown>>;
}
```

これならPlacementは、

> いつ、どのLaneに存在するか

だけを担当する。

Render Nodeは、

> 何を、どう描画・合成・生成するか

を担当する。

同じRender Nodeを複数Placementで参照してもいいし、Nested CompositionやAdjustment Layerも自然に表せる。

---

# 5. Effect Listではなく軽量なNode Graphを持つ

単なるEffectの縦積みだけでは、次のような処理が弱い。

```text
映像A ─ Crop ─┐
              ├─ Split Screen ─ Color Grade
映像B ─ Crop ─┘
```

```text
Voice ────────┐
              ├─ Compressor ─ Master
BGM ─ Duck ───┘
```

フルノードコンポジタを最初から作る必要はないけれど、内部的には、

```text
Source
Transform
Mask
Effect
Composite
Mix
Output
```

くらいの型付きNode Graphが欲しい。

TimelineのTrackはUI上の編集構造。

Node Graphはレンダリング上の構造。

ここは分けるべきね。

---

# 6. 編集操作を「Selectorに対する変換」として定義する

VELの実行層は、こういう形がいい。

```text
Selector
   ↓
EntitySet / SpanSet / RegionSet
   ↓
Operator
   ↓
Edit Patch
   ↓
Project IR
```

例えば完全に非意味的な編集なら、

```vel
track("V1").clips
  |> where(.duration < 500ms)
  |> delete(ripple: true)
```

```vel
track("broll").clips
  |> every(3)
  |> transform.scale(1.08)
```

```vel
range(12s..18s) on track("V1")
  |> retime(0.5)
  |> animate(
       property: transform.scale,
       from: 1.0,
       to: 1.25,
       duration: 12f,
       easing: ease_out
     )
```

```vel
region rect(0.78, 0.02, 0.20, 0.15) in track("V1")
  |> blur(radius: 18px)
```

```vel
bus("music").gain
  |> modulate(
       by: bus("voice").rms,
       using: duck(
         threshold: -32dB,
         amount: -8dB,
         attack: 80ms,
         release: 240ms
       )
     )
```

どれもTranscriptやScene Recognitionには依存していない。

---

# 7. 宣言型だけでなく、限定的な実行層が必要

ここも前の判断を少し撤回した方がいい。

私は先ほど、

> JavaScript的な実行ロジックを混ぜない方がいい

と言ったけれど、あれは半分正しく、半分やりすぎだった。

Project IRそのものは宣言型でいい。

しかしVELという**編集言語**には、反復やSelectionへの一括適用が必要よ。

たとえば、

```vel
for clip in track("montage").clips {
  clip.fade_in = 4f
  clip.fade_out = 4f
}
```

や、

```vel
track("images").clips
  |> stagger(by: 12f)
  |> layout(grid(columns: 3))
```

といった処理は、非意味的編集のかなり大きな部分になる。

ただし一般的な`while`や任意I/Oまで入れる必要はない。

SQLやCSSや関数型Pipelineに近い、

```text
選ぶ
並べ替える
変換する
集約する
繰り返す
```

という限定的な実行モデルで十分でしょう。

---

# 改訂した内部構造

```text
Project Core
├─ Media Catalog
│  └─ Asset / Stream
│
├─ Composition
│  ├─ Sequence
│  ├─ Lane / Layer
│  ├─ Placement
│  ├─ TimeMap
│  └─ Nested Composition
│
├─ Render Graph
│  ├─ Source Node
│  ├─ Transform Node
│  ├─ Mask Node
│  ├─ Effect Node
│  ├─ Composite Node
│  └─ Output Node
│
├─ Automation
│  ├─ Property<T>
│  ├─ Curve<T>
│  ├─ Expression<T>
│  └─ Constraint
│
├─ Spatial Model
│  ├─ CoordinateSpace
│  ├─ Geometry
│  ├─ Region
│  └─ RegionTrack
│
├─ Audio Graph
│  ├─ Bus
│  ├─ Route
│  ├─ Send
│  └─ Gain / Pan / Automation
│
└─ Edit History
   └─ Patch / Operation

Optional Sidecar
├─ EventSet
├─ SpanSet
├─ Signal
├─ RegionTrack
└─ Annotation
```

これならProject Coreは、Analysis Sidecarが一切なくても完全に編集・表示・レンダリングできる。

---

# 残すものと捨てるもの

前の案から、そのまま残していいのは、

- Asset / Stream
- Sequence
- TimeMap
- RationalTime
- Patch
- Materializedな配置値を持つ方針

あたり。

中心から降ろすべきなのは、

- AnalysisFeature
- Semantic Selection
- Anchor / Binding中心の編集モデル

追加すべき核は、

- Selector
- Placement
- Render Node
- Property / Curve
- CoordinateSpace / Region
- Signal
- Audio Bus
- Constraint

になる。

特に大きな修正はこれね。

```text
旧：
Analysis
  ↓
Anchor
  ↓
Timeline Item

新：
Entity / Time / Space / Signal
  ↓
Selector
  ↓
Operator
  ↓
Composition + Render Graph
```

Transcriptやシーン認識は、この新しい系では単に便利なSelection入力にすぎない。

この形ならVELは、

> AIに見どころを切らせる言語

ではなく、

> **動画の時間構造・空間構造・信号・描画構造を、コードとして操作する言語**

になる。

そっちでないと弱いわ。次に型を切るなら、`Placement / Property<T> / Curve<T> / EventSet / SpanSet / Signal<T> / RegionTrack / Selector<T>`の八つから始めるのが筋でしょう。

---

# you asked

message time: 2026-08-19 00:21:47

編集環境のイメージも難しいな。こう、決定論的に記述できるIDE的なやつも欲しい

---

# chatgpt response

そうね。ここはかなり重要で、**VELの言語設計だけ考えていても編集環境の像が立たない理由**があると思う。

たぶん欲しいのは「動画編集ソフトにコード欄を付ける」じゃないのよ。

> **動画プロジェクトそのものをプログラムとして扱えるVideo IDE**

に近い。

## 事実

通常のNLEは、タイムライン上のGUI操作が一次情報になる。

```text
ドラッグする
↓
内部状態が変わる
↓
プロジェクトファイルに保存
```

一方、IDE的な環境では普通、

```text
Source Code
↓
Compiler / Runtime
↓
Program State / Output
```

という方向がある。

VELで決定論的な編集を成立させたいなら、基本構造も後者に寄せる必要があるわ。

つまり、

```text
VEL Source
   ↓
Resolve / Compile
   ↓
Project Graph
   ↓
Preview / Render
```

が大黒柱になる。

---

# 推測（示唆）

私は編集環境を、**4つのビューが同じProject Graphを見ているIDE**として考えると急に像が立つと思う。

```text
┌──────────────────────────────────────────────┐
│ Project / Assets │           Preview         │
│                  │                           │
│ capture.mp4      │      ┌────────────┐       │
│ voice.wav        │      │            │       │
│ intro.vel        │      │    VIDEO   │       │
│                  │      │            │       │
│                  │      └────────────┘       │
├──────────────────┼───────────────────────────┤
│                  │                           │
│   VEL Editor     │        Timeline           │
│                  │                           │
│ clips(...)       │ V2 ───────█────────       │
│   |> trim(...)   │ V1 ████ ███████ ███       │
│                  │ A1 █████████████████       │
│                  │                           │
├──────────────────┴───────────────────────────┤
│ Problems │ Inspect │ History │ Render │ REPL │
└──────────────────────────────────────────────┘
```

Visual Studio CodeとPremiereとAfter Effectsを事故らせたようなものね。

ただし重要なのは見た目ではなく、**どのビューも同じものを編集している**こと。

---

# Sourceが一次情報

例えば、

```vel
sequence main {
    place gameplay
        on video.main
        at 0s
        source 10s..35s

    place title
        on overlay
        at 2s
        duration 3s
}
```

がある。

Timelineには当然、

```text
V2        [ title ]
V1 [       gameplay        ]
```

と出る。

ここでTimeline側からtitleを1秒右にドラッグする。

普通のNLEなら内部データだけ変わる。

VEL IDEでは、

```diff
 place title
     on overlay
-    at 2s
+    at 3s
     duration 3s
```

になる。

これがかなり重要だと思う。

**GUI操作がSource transformationになる。**

---

# ただ、全部をテキストへ即座に逆変換するのも危険

ここで難所が出てくる。

例えば、

```vel
track("images").clips
    |> stagger(by: 12f)
```

によって10個のClipを配置したとする。

Timeline上で7個目だけ人間がドラッグしたら、

元のコードをどう変える？

単純に、

```vel
clip("img7").start += 4f
```

を追加する方法もある。

つまり、

```vel
track("images").clips
    |> stagger(by: 12f)

override clip("img7") {
    start += 4f
}
```

みたいになる。

これ、かなり良いと思う。

---

# 「生成」と「Override」の二層

VEL IDEは、

```text
Rule
↓
Generated State
↓
Manual Overrides
↓
Effective State
```

にすると強そう。

例えば、

```vel
rule montage {
    select track("screenshots").clips
    arrange sequential
    duration each 2s
    transition crossfade(6f)
}
```

で自動配置する。

でも3枚目だけ、

```vel
override clip("shot-3") {
    duration 3.5s
    transform.scale 1.15
}
```

にする。

すると人間がGUIで触っても、**元の規則を破壊しない**。

これはCSSにちょっと似ているわね。

---

# GUIで触った箇所がコード上で光る

IDEっぽさを強くするなら、これが欲しい。

Timeline上でClipをクリックすると、

```vel
place boss_fight {
    source capture[12:31.400 .. 12:48.200]
    at main.after(intro)
}
```

がEditor側で選択される。

逆にコードで、

```vel
transform.scale
```

へカーソルを置くとPreviewにBounding Boxが出る。

```text
Code
 ↕
Timeline
 ↕
Preview
```

が完全に同期する。

これはNLEにCode Editorを足したのではなく、**ASTの複数Projection**と考えた方が綺麗。

---

# Preview上でも直接編集する

例えば映像中のテキストをドラッグ。

```text
Preview上でタイトルを右へ動かす
```

すると、

```diff
 transform {
-    position (0.50, 0.85)
+    position (0.63, 0.85)
 }
```

になる。

あるいはKeyframeをPreview上で打つ。

```vel
transform.position {
    at 0s    = (0.2, 0.5)
    at 500ms = (0.8, 0.5) ease-out
}
```

Timeline、Preview、VEL Editorが全部**同じProperty Graphを編集するUI**になる。

ここは相当気持ちいいと思う。

---

# `vel`と`vel.lock`を分ける案

決定論性との相性がすごくいい。

例えば人間が書くのは、

```vel
select highlights
    where motion > 0.7

arrange highlights
    target-duration 30s
```

だけ。

ただしこれだけでは、解析結果が変わればTimelineも変わる。

そこで、

```text
main.vel
main.vel.lock
```

を持つ。

`main.vel`：

```vel
highlights = select clips
    where motion > 0.7
```

`main.vel.lock`：

```text
highlights:
  - capture.mp4@103.400..108.700
  - capture.mp4@221.100..227.900

analysis:
  motion-model: motion-v3
  media-hash: abc123...
```

とする。

通常のPreview/Renderはlock済み結果を使う。

明示的に、

```text
VEL: Resolve
```

したときだけ再評価。

これは`package.json`と`package-lock.json`に近い。

かなり綺麗よ。

---

# IDEには「Deterministic / Dynamic」の境界を見せたい

ここが個人的には重要。

例えばEditor上で、

```vel
place intro at 0s
```

は完全に決定的なので普通に表示。

一方、

```vel
select using vision "good combat scenes"
```

はDynamic。

IDEで、

```text
✓ deterministic
◇ resolved
⚠ unresolved
```

のように区別する。

たとえば、

```vel
◇ highlights = vision.select(
      "visually clear examples of the mechanic"
  )
```

をクリックすれば、

```text
Resolved by:
  model: xxx
  media: capture.mp4@sha256:...
  output: 7 ranges
  resolved: 2026-08-19

[Re-resolve]
```

がInspectorに出る。

**どこまでがプログラムで、どこからが推論なのかを隠さないIDE**。

これはVELらしさとしてかなり良いと思う。

---

# REPLも欲しい

動画編集は探索的だから、

```text
> select clips where duration < 1s
12 clips

> preview selected

> selected |> ripple_delete
estimated duration: 8:41 → 8:33

> apply
```

みたいな操作が欲しい。

あるいは、

```text
> track("voice").silence(min: 800ms)
37 spans, total 51.2s

> highlight
```

だけ実行してTimeline上で確認。

これならコードを書き切らなくてもいい。

SQL Console的な編集環境ね。

---

# Scratch Layerもあるといい

さらに私は、

```text
Committed Project
+
Scratch Edits
```

を分けたい。

例えばGUIで好き勝手触る。

```text
5 manual edits
2 experimental effects
1 temporary cut
```

IDE下部に、

```text
Uncommitted edits
```

として出る。

気に入ったら、

```text
Commit as overrides
```

または、

```text
Refactor into rule
```

をする。

後者がかなり面白い。

人間がTimelineで10個のClipを全部2秒にしたあと、

> **Refactor this into VEL rule**

とすると、

```vel
track("screenshots").clips
    |> set duration 2s
```

へまとめる。

これは普通のIDEでいう、

> Extract Function

の動画編集版よ。

---

# AgentもこのIDEの同僚になる

AgentにはProjectファイルを直接好き放題書き換えさせず、

```text
Agent
 ↓
Patch
 ↓
Diff View
 ↓
Project
```

がいい。

例えば、

> 「冒頭30秒を少しテンポ良くして」

で、

```diff
main.vel
@@ intro

+ override clip("intro-03") {
+   source.in += 480ms
+ }

- gap 820ms
+ gap 240ms

+ transition between intro-04, intro-05 {
+   crossfade 4f
+ }
```

みたいな普通のコードレビューになる。

そして横に動画Preview。

**Git diffを見ながら映像差分を確認する。**

これはかなりVideo IDE感がある。

---

# 「Video Diff」まで行ける

コードのdiffだけでは分かりにくいので、

```text
Before            After
┌─────────┐       ┌─────────┐
│         │       │         │
└─────────┘       └─────────┘

duration   -1.42s
clips      12 → 13
cuts       +2
effects    +1
```

さらにTimeline差分を、

```text
        removed
           ↓
V1 ███████-----███████
             ↑
           moved 1.2s
```

みたいに見せる。

そうするとAgentの編集をちゃんとレビューできる。

---

# たぶんUIの中心はTimelineですらない

ここは結構踏み込みたい。

従来NLEではTimelineが絶対的中心だけれど、VEL IDEなら、

```text
Source ↔ Structure ↔ Timeline ↔ Preview
```

の4つが対等でもいい。

例えばStructure View：

```text
main
├─ intro
│  ├─ title
│  └─ gameplay
├─ explanation
│  ├─ voice
│  ├─ gameplay
│  └─ captions
└─ conclusion
```

これはDOM TreeやAST Explorerに近い。

長尺動画になると、100個のClipが横に並ぶTimelineよりこっちの方が編集しやすい場面もあるでしょう。

---

# 一番欲しい像

最終的にはこんな感じね。

```text
                 VEL PROJECT

         ┌────── Source ──────┐
         │                     │
 Human ──┼─→ VEL / Rules       │
         │                     │
 Agent ──┼─→ Patches           │
         │                     │
 GUI ────┼─→ Overrides         │
         │                     │
         └─────────┬───────────┘
                   ↓
             Project Graph
                   ↓
       ┌───────────┼──────────┐
       ↓           ↓          ↓
    Timeline    Preview    Structure
       │           │          │
       └────── edits back ────┘
                   ↓
             Deterministic
                 Render
```

そして私はこの方向なら、**VELの正体を「動画編集DSL」より「動画編集用のprogramming environment」と考え直した方がいい**気がしてきた。

Languageだけでは片翼なのよ。

`VEL + Project Graph + Resolver + IDE`

まで合わせて初めて一つのシステムになる。

特に **「GUIで雑に作る → diffを見る → ruleへrefactorする」** ができるなら、決定論的記述とNLE的な直接操作を無理に二者択一にしなくて済む。ここ、かなり有望だと思うわ。

---

# you asked

message time: 2026-08-19 00:25:48

名前考えるか。エコシステムそのものの

---

# chatgpt response

そうね。**VELは言語名としては説明的でいいけれど、エコシステム全体の名前には少し機能名っぽすぎる**わ。

## 事実

今まで立ち上がってきたものは、単なる動画編集DSLではなく、

```text
Ecosystem
├─ Language / DSL
├─ Project Graph
├─ Resolver
├─ Compiler
├─ Render backend
├─ IDE
├─ CLI
├─ Agent interface
└─ Package / plugin ecosystem
```

くらいの広がりがある。

だから名前も「動画を切る」より、

> **映像をプログラム可能な媒体として扱う**

くらいの抽象度を持っていた方が長生きすると思う。

---

## 推測（示唆）

今のところ私は、名前の方向性を4系統に分けたい。

### 1. **Continuum**

かなり好き。

動画そのものが時間的連続体なので意味も自然だし、

```text
Continuum
├─ Continuum Language
├─ Continuum Studio
├─ Continuum CLI
└─ Continuum Runtime
```

と製品群を展開しやすい。

ただ、一般語すぎて検索性や既存製品との衝突はかなり怪しい。そこは後で確認が必要ね。

---

### 2. **Splice**

編集そのものを強く想起する。

```text
Splice
Splice Studio
Splice Language
splice build
splice render
```

CLIも気持ちいい。

ただし「動画編集」側にかなり寄るので、将来Compositionや生成映像、インタラクティブ映像まで広げると少し狭く感じる可能性がある。

あとこれは明らかに既存サービスがありそうな名前なので、ブランド名としては要調査。

---

### 3. **Timelineという概念から離れる名前**

私はむしろこの方向が面白い。

#### **Lattice**

格子、関係構造。

VELが、

```text
素材
時間
空間
信号
Rule
Binding
```

を結ぶProject Graphだと考えるとかなり合う。

```text
Lattice Studio
Lattice IR
Lattice Runtime
Lattice Agent
```

いい響き。

「動画を一本の線としてではなく、関係のネットワークとして扱う」という思想まで名前に入る。

これはかなり本命候補。

---

### 4. **映像をコード化する思想から取る**

#### **Glyph**

記号・表現。

```text
Glyph Studio
Glyph Language
Glyph Project
```

映像を記号体系に変える感じがある。

かなり綺麗だけど、やや抽象的ね。

#### **Framecode**

意味は一発で分かる。

ただし少しGitHubプロジェクト臭が強い。

#### **Reel**

短くて強い。

```text
Reel
Reel Studio
ReelScript
```

ただ「古いフィルム」のニュアンスが強いし、名前の衝突も激しそう。

---

# 私が今一番好きなのは **Lattice**

理由は、今の思想そのものにかなり合っているから。

普通のNLEは、

```text
time ─────────────────────→
```

を中心にする。

でも今考えているものは、

```text
          Asset
          /   \
       Clip   Analysis
       /  \       \
    Time  Space   Signal
      \     |      /
       \   Rules  /
        Project Graph
```

なのよ。

つまりTimelineではなく**関係網が本体**。

それをIDEから複数のProjectionとして、

```text
Code
Timeline
Preview
Graph
Structure
```

で見る。

だったら「Lattice」という名前はかなり本質を突いている。

---

もう少し尖らせるなら、私はこの辺も好き。

- **Lattice** — 構造・関係性。最有力。
- **Continuum** — 時間媒体として美しい。
- **Skein** — 糸の束。複数の時間線や素材が絡む感じ。
- **Weave** — 映像・音声・字幕を編む。親しみやすい。
- **Flux** — 時間変化・信号・変換。技術的で強い。
- **Facet** — 同一ProjectをCode/Timeline/Previewなど複数面から見る。
- **Relay** — Agent、人間、Compiler間で編集意図を受け渡す。
- **Strata** — Layerや抽象層を重ねる。
- **Kine** — kinetic/cinema由来っぽい短さ。
- **Vellum** — 書かれた媒体感があり、VELとも少し響く。

個人的には、

> **Lattice — programmable video environment**

くらいまで、もうかなり好きだわ。

そして言語そのものは無理に同名にせず、

```text
Lattice        ecosystem
Lattice Studio IDE
VEL            language
Lattice IR     project representation
lattice        CLI
```

でもいい。

むしろ**VELという無骨な技術名が内部に残っていて、外側だけLatticeというブランドを被る**の、OSSプロジェクトとしてかなり自然でしょう。

---

# you asked

message time: 2026-08-19 00:27:38

じゃあそれで。
最悪手書きできる程度まで文法を整理するか

---

# chatgpt response

そうね。ここで一回、**「IDEが全部生成してくれる前提」を捨てて、`.vel`を素手で書いて一本作れる**ところまで落としたいわ。

私なら、VEL v0 の表面文法はかなり小さくする。

## 設計方針

核はこの二つだけでいい。

```text
1. 具体的な編集状態を宣言する
2. その状態に決定論的なRuleを掛ける
```

つまり、

```text
Assets
  ↓
Sequences / Tracks / Clips
  ↓
Rules
  ↓
Overrides
  ↓
Project Graph
```

AI、文字起こし、シーン認識は**Core文法には入れない**。後で`resolve`系として載せればいい。

そして文法上の約束はかなり保守的にする。

- `{}`でブロック
- `=`で値設定
- 改行区切り、`;`不要
- `//`コメント
- 時間や座標は単位付き
- 任意の`for / while`は持たない
- 一括操作はSelector + Pipeline
- 暗黙挙動は極力少なくする
- `lattice fmt`で必ずCanonical Formに戻せる

---

# まず一本

こんな感じなら、私は普通に手で書けると思う。

```vel
project "obex-demo" {
  fps = 60
  size = 2560x1440
  sample_rate = 48000Hz

  asset game = media("./capture.mp4")
  asset voice = media("./voice.wav")
  asset logo = image("./logo.png")
  asset bgm = media("./bgm.flac")

  sequence main {
    marker hook = 3s

    track video v1 flow {
      clip intro = game.video[10s..18s]

      gap 12f

      clip demo = game.video[41s..57s] {
        speed = 1.0x
      }
    }

    track audio a1 flow {
      clip intro_audio = game.audio[10s..18s]

      gap 12f

      clip demo_audio = game.audio[41s..57s]
    }

    track video overlay free {
      clip title = text("Prisma Obex") at hook for 2.5s {
        transform.position = (50%, 84%)

        opacity = curve {
          0f:       0
          8f:       1 ease-out
          end - 8f: 1
          end:      0 ease-in
        }
      }

      clip watermark = logo at 1s for 4s {
        transform.position = (92%, 8%)
        transform.scale = 0.18
      }
    }

    track audio music free {
      clip music = bgm.audio at 0s {
        gain = -14dB
      }
    }
  }

  rule remove_tiny_clips {
    main.v1.clips
      |> where(duration < 500ms)
      |> delete()
  }

  override main.v1.demo {
    speed = 0.5x
  }

  export youtube {
    from = main
    size = 2560x1440
    fps = 60
    codec = h265
  }
}
```

これくらいなら、IDEが死んでいても何をやっているか分かる。

---

# `flow` と `free`

ここはかなり重要な文法上の発明になりそう。

## Flow Track

```vel
track video v1 flow {
  clip a = game.video[10s..15s]
  clip b = game.video[20s..24s]
  gap 500ms
  clip c = game.video[30s..35s]
}
```

これは、

```text
[a][b]--500ms--[c]
```

になる。

つまり**ソースコード上の順序が時間順序**。

`b`を消せば、

```text
[a]--500ms--[c]
```

になる。

Ripple Editが特殊操作ではなく、**データ構造そのもの**になるわ。

長尺の本編トラックはこれでかなり書きやすい。

---

## Free Track

一方、

```vel
track video overlay free {
  clip title = text("Hello") at 2s for 3s
  clip logo  = logo at 12s for 5s
}
```

では絶対・参照時刻で配置する。

```text
time ─────────────────────→
        [title]
                  [logo]
```

こっちは字幕、Overlay、BGM、効果音などに向く。

この二つを分けるだけで、

> 「全部のClipに`at previous.end`を書く」

みたいな不毛さを消せる。

---

# 時間リテラル

ここは徹底して単純にする。

```vel
500ms
2s
1.25s

1m
1m23.5s

4f
120f
```

FrameはSequenceのFPS基準。

```vel
8f
```

なら60fps Sequenceでは8/60秒。

時間式も普通に書ける。

```vel
hook + 12f
intro.end
intro.end + 500ms
title.start - 4f
```

なので、

```vel
clip popup = text("!") at demo.start + 12f for 2s
```

も成立する。

---

# Range

素材範囲は、

```vel
game.video[10s..18s]
```

だけ。

```text
10s <= t < 18s
```

というhalf-open rangeに固定したい。

動画編集でendpointのinclusive/exclusiveを曖昧にするのは、後から一フレームの妖怪が出るからね。

フレームでもいい。

```vel
game.video[120f..480f]
```

---

# 単位は型として扱う

単なるnumberにしない。

```vel
500ms
8f
-6dB
48px
50%
1.25x
48000Hz
```

したがって、

```vel
gain = 400px
```

はコンパイルエラー。

```vel
transform.position = (50%, 80%)
```

は通る。

この辺はIDEの補完にも効く。

---

# Clip

基本形は一つだけ。

```vel
clip <id> = <content> [at <time>] [for <duration>] {
  ...
}
```

例えば、

```vel
clip gameplay = game.video[10s..20s]
```

素材Rangeからdurationを推論できる。

静止画なら、

```vel
clip screenshot = image("./shot.png") for 3s
```

Free Trackなら、

```vel
clip screenshot = image("./shot.png") at 12s for 3s
```

。

Inline生成物も同じ。

```vel
clip title = text("Hello") at 2s for 3s
```

ここは「Video」「Image」「Text」を別構文にしない方がいい。

全部**Clipに配置できるContent**で統一する。

---

# Property

Clipの中身はほぼ全部、

```vel
property = expression
```

にする。

```vel
transform.position = (50%, 50%)
transform.scale = 1.2
transform.rotation = 4deg

opacity = 0.8

gain = -6dB
pan = -0.25

speed = 0.5x
```

Property専用の文法を増やさない。

これならPlugin Propertyも普通に追加できる。

```vel
filmgrain.amount = 0.12
```

---

# Curve

Keyframeも特殊構文を最小化する。

```vel
opacity = curve {
  0f:       0
  8f:       1 ease-out
  end - 8f: 1
  end:      0 ease-in
}
```

Positionも同じ。

```vel
transform.position = curve {
  0f:  (20%, 50%)
  12f: (50%, 50%) ease-out
}
```

色だろうが音量だろうが同じ。

```vel
gain = curve {
  0s:  -12dB
  3s:  -6dB
  end: -12dB
}
```

つまりVEL内部の、

```text
Property<T>
```

がそのまま表面文法にも出てくる。

これはかなり綺麗。

---

# Effectも関数として扱える

例えば、

```vel
effects = [
  blur(radius: 12px),
  sharpen(amount: 0.2)
]
```

でもいいし、読みやすさが必要なら後から、

```vel
effect blur {
  radius = 12px
}
```

というSugarを追加できる。

ただ、**v0では前者だけでも十分**だと思う。

新しいEffectごとに文法を増やしたくないから。

---

# Selector + Rule

そしてコード編集らしい部分。

```vel
rule remove_tiny_clips {
  main.v1.clips
    |> where(duration < 500ms)
    |> delete()
}
```

ここにはTranscriptもAIもいない。

単なるProject GraphへのQuery。

他にも、

```vel
rule speed_up_broll {
  main.broll.clips
    |> where(duration > 5s)
    |> retime(1.25x)
}
```

とか、

```vel
rule zoom_every_third {
  main.broll.clips
    |> every(3)
    |> apply {
      transform.scale = 1.08
    }
}
```

。

Selectorで扱えるものは、

```text
clips
tracks
markers
effects
transitions
regions
properties
```

くらいにする。

---

# Pipelineは一般言語にしすぎない

v0では、

```vel
something
  |> where(...)
  |> sort(...)
  |> first(...)
  |> every(...)
  |> apply {...}
  |> delete()
  |> trim(...)
  |> retime(...)
  |> move(...)
```

くらい。

`map`や`reduce`さえ最初はいらないと思う。

動画編集に必要な操作を名前付きPrimitiveとして増やす。

つまりVELは、

```text
JavaScript + video API
```

にはしない。

そうするとAgentが突然、

```vel
while true {
```

を書き始める事故も防げる。

---

# Override

GUI直接操作との接点。

例えばRuleによって配置された`demo`だけ変えたい。

```vel
override main.v1.demo {
  speed = 0.5x
  transform.scale = 1.05
}
```

相対的な操作も必要なら、

```vel
override main.v1.demo {
  move(by: 4f)
}
```

とする。

これなら、

```text
Base declarations
↓
Rules
↓
Overrides
```

の順で評価できる。

GUIでClipを4フレーム動かしたら、IDEはこの`move(by: 4f)`を生成すればいい。

---

# Marker

Semantic Anchorを特別扱いせず、まず素朴なMarkerを持つ。

```vel
marker hook = 3s
marker boss = gameplay.start + 12s
```

そして普通に、

```vel
clip popup = text("Now!") at boss + 4f for 2s
```

。

後から、

```text
manual marker
beat detection
transcript hit
vision event
```

全部を同じReference体系へ接続すればいい。

---

# コメント

普通。

```vel
// このカットだけ少し長く残す
clip demo = game.video[41s..57s]
```

複数行文字列も欲しい。

```vel
clip card = text("""
This is a longer
piece of text.
""") at 12s for 4s
```

---

# 最小文法

かなり雑なEBNFにすると、Coreはこの程度で済む。

```text
file
  = project ;

project
  = "project" STRING block ;

block
  = "{" statement* "}" ;

statement
  = assignment
  | asset_decl
  | sequence_decl
  | track_decl
  | clip_decl
  | gap_decl
  | marker_decl
  | rule_decl
  | override_decl
  | export_decl
  | pipeline ;

asset_decl
  = "asset" IDENT "=" expression ;

sequence_decl
  = "sequence" IDENT block ;

track_decl
  = "track" TRACK_KIND IDENT TRACK_MODE block ;

TRACK_KIND
  = "video"
  | "audio"
  | "caption" ;

TRACK_MODE
  = "flow"
  | "free" ;

clip_decl
  = "clip" IDENT "=" expression
    [ "at" expression ]
    [ "for" expression ]
    [ block ] ;

gap_decl
  = "gap" expression ;

marker_decl
  = "marker" IDENT "=" expression ;

rule_decl
  = "rule" IDENT block ;

override_decl
  = "override" path block ;

export_decl
  = "export" IDENT block ;

assignment
  = path "=" expression ;

pipeline
  = expression ( "|>" call )+ ;
```

Expression側は、

```text
literal
identifier
property access
range
tuple
list
function call
curve
+ - * /
< <= == != >= >
and or not
```

くらいでいい。

---

# 意図的に入れないもの

v0では、

```text
for
while
class
async
await
filesystem access
network access
random
current_time()
eval
user-defined imperative functions
```

全部なし。

VEL Sourceだけで実行した場合は、**常に同じProject Graphになる**。

そして将来AIを入れるときだけ、

```text
resolve
```

という別の境界を作る。

```text
決定論的VEL
      ↑
 resolved artifact
      ↑
 Resolver / VLM / LLM
```

にした方がいい。

---

## 私ならここで一度固定する

今は文法を増やさず、まずこのサブセットだけ。

```text
project
asset

sequence
track flow/free
clip
gap
marker

property = expr
curve

rule + pipeline
override

export
```

**12個程度しか概念がない。**

その代わり、これだけで実際に5分くらいの動画を記述してみる。

そこで、

> 「この編集、VELでは異様に書きづらい」

となった場所から文法を増やす。

特に`flow/free`はかなり効くと思うわ。これがあるとVELが単なる「JSONをちょっと綺麗にしたもの」から、**動画編集そのものを人間が記述できる言語**に一段近づく。

---

# you asked

message time: 2026-08-19 00:35:24

実際にどんな編集をしたいか、ゲーム実況解説を想定して今のVELで書こうとしてみて

---

# chatgpt response

やってみる価値あるわ。今回は「文法を褒める」んじゃなくて、**8分くらいのゲーム実況解説を本当に一本作るつもりでVELに押し込み、書きづらい箇所を欠陥として拾う**方向で見る。

想定する編集は、単に「実況を切る」より少し現実的にするわ。**見せ場のコールドオープン→導入→ビルド画面解説→実戦→一時停止して注釈→比較→リプレイ→締め**まで入れる。これなら時間・空間・音・再利用・同期が全部殴れる。

うん。**ここで一度「理想の言語」じゃなく、実際に一本編集する側へ降りる**のがいいわ。

想定は、たとえば「Prisma Obexの仕組み・ビルド・実戦性能を6〜8分で解説する動画」。実況素材を後から編集して解説動画に仕立てる感じで考える。

## 事実

まず、実際にやりたくなりそうな編集を並べるとこうなる。

- 冒頭に一番派手な実戦シーンを8秒
- タイトルをフェードイン
- 本編へハードカット
- 実況音声とは別に録ったナレーションを流す
- ナレーション中もゲーム映像はどんどん差し替える
- Arsenal画面を見せる
- 特定部分を拡大して注釈
- 移動だけの部分は2.5倍速
- 大ダメージが出た瞬間で一時停止
- 画面内の数字を拡大＋枠＋テキスト
- その直前をもう一度リプレイ
- 2種類のビルドを左右同時再生して比較
- ナレーション中はBGMを下げる
- 最後にまとめとOutro

これを**今まで決めたVELだけ**で無理やり書いてみる。

---

### 現VELでの第一稿

```vel
project "prisma-obex-guide" {
  fps = 60
  size = 2560x1440
  sample_rate = 48000Hz

  asset game  = media("./capture.mp4")
  asset voice = media("./narration.wav")
  asset bgm   = media("./bgm.flac")

  asset build_a = image("./build-a.png")
  asset build_b = image("./build-b.png")

  sequence main {

    //
    // MAIN VIDEO
    //

    track video v1 flow {

      // 0:00 - cold open
      clip hook = game.video[26m14s..26m22s]

      // introduction background
      clip intro = game.video[1m40s..1m58s]

      // build explanation background
      clip arsenal = game.video[5m10s..5m32s]

      // first demonstration
      clip demo_a = game.video[12m04s..12m21s]

      // boring movement
      clip traversal = game.video[12m21s..12m35s] {
        speed = 2.5x
      }

      // actual attack
      clip impact = game.video[12m35s..12m43s]

      // replay
      clip replay = game.video[12m34s..12m43s] {
        speed = 0.65x
      }

      // second demonstration
      clip demo_b = game.video[18m12s..18m34s]

      // comparison section background
      clip compare_base = game.video[22m01s..22m09s]

      // conclusion
      clip outro = game.video[28m20s..28m38s]
    }


    //
    // GAME AUDIO
    //
    // ここで早くもかなり嫌な臭いがする。
    //

    track audio game_audio flow {
      clip hook_audio = game.audio[26m14s..26m22s] {
        gain = -5dB
      }

      clip intro_audio = game.audio[1m40s..1m58s] {
        gain = -12dB
      }

      clip arsenal_audio = game.audio[5m10s..5m32s] {
        gain = -16dB
      }

      clip demo_a_audio = game.audio[12m04s..12m21s] {
        gain = -10dB
      }

      clip traversal_audio = game.audio[12m21s..12m35s] {
        speed = 2.5x
        gain = -18dB
      }

      clip impact_audio = game.audio[12m35s..12m43s] {
        gain = -8dB
      }

      clip replay_audio = game.audio[12m34s..12m43s] {
        speed = 0.65x
        gain = -10dB
      }

      clip demo_b_audio = game.audio[18m12s..18m34s] {
        gain = -10dB
      }

      clip compare_audio = game.audio[22m01s..22m09s] {
        gain = -12dB
      }

      clip outro_audio = game.audio[28m20s..28m38s] {
        gain = -12dB
      }
    }


    //
    // NARRATION
    //

    track audio narration free {
      clip narration = voice.audio at intro.start {
        gain = -2dB
      }
    }


    //
    // MUSIC
    //

    track audio music free {
      clip music = bgm.audio at intro.start {
        gain = -18dB
      }
    }


    //
    // TITLE / BUILD GRAPHICS
    //

    track video graphics free {

      clip title = text("Prisma Obex Incarnon") at hook.start + 2s for 3s {
        transform.position = (50%, 78%)

        opacity = curve {
          0f:       0
          8f:       1 ease-out
          end - 8f: 1
          end:      0 ease-in
        }
      }


      // Arsenal screenshot
      clip build_screen = build_a at arsenal.start + 2s for 14s {
        transform.scale = 0.88
        transform.position = (50%, 50%)
      }


      // screenshotの特定部分へズーム
      clip build_zoom = build_a at arsenal.start + 9s for 5s {
        transform.scale = curve {
          0f:  0.88
          12f: 1.65 ease-out
        }

        transform.position = curve {
          0f:  (50%, 50%)
          12f: (67%, 38%) ease-out
        }
      }


      clip build_note =
        text("ここでFinisher倍率を伸ばす")
        at arsenal.start + 10s
        for 4s
      {
        transform.position = (68%, 72%)
      }


      //
      // impact直後の説明
      //

      clip damage_note =
        text("この一撃に注目")
        at impact.start + 5s
        for 2s
      {
        transform.position = (52%, 24%)
      }


      //
      // replay表示
      //

      clip replay_label =
        text("REPLAY")
        at replay.start
        for 1.5s
      {
        transform.position = (10%, 10%)
      }


      //
      // outro
      //

      clip conclusion =
        text("結論：フィニッシャー用途なら十分過剰火力")
        at outro.start + 2s
        for 6s
      {
        transform.position = (50%, 78%)
      }
    }


    //
    // COMPARISON
    //

    track video compare_left free {
      clip left =
        game.video[22m01s..22m09s]
        at compare_base.start
        for 8s
      {
        transform.scale = 0.5
        transform.position = (25%, 50%)
      }
    }

    track video compare_right free {
      clip right =
        game.video[24m12s..24m20s]
        at compare_base.start
        for 8s
      {
        transform.scale = 0.5
        transform.position = (75%, 50%)
      }
    }

    track video compare_labels free {
      clip label_a =
        text("Build A")
        at compare_base.start
        for 8s
      {
        transform.position = (25%, 8%)
      }

      clip label_b =
        text("Build B")
        at compare_base.start
        for 8s
      {
        transform.position = (75%, 8%)
      }
    }
  }


  export youtube {
    from = main
    size = 2560x1440
    fps = 60
    codec = h265
  }
}
```

これ、**意外と読める**。

しかも文字起こしもVLMも一切使っていない。

ただ、実際に書いたことでかなり露骨に欠陥が見えてきた。

---

# 1. 一番まずいのはA/V同期

ここ。

```vel
clip impact = game.video[12m35s..12m43s]
```

に対して、

```vel
clip impact_audio = game.audio[12m35s..12m43s]
```

をもう一度書いている。

これは絶対嫌。

10回Cutしたら20箇所編集することになる。

さらに、

```vel
speed = 0.65x
```

もVideoとAudioの両方に書く。

**手動同期バグ製造機**よ。

ゲーム録画は基本、

```text
Media
├─ Video Stream
└─ Audio Stream
```

なのだから、表面言語ではまず、

```vel
clip impact = game[12m35s..12m43s]
```

と書きたい。

これで、

```text
linked video placement
linked audio placement
```

を生成する。

必要なときだけ、

```vel
clip impact = game[12m35s..12m43s] {
  video {
    ...
  }

  audio {
    gain = -8dB
  }
}
```

と個別設定。

これは今回かなり強く見えた。

---

# 2. 「Clip」より「Section」が欲しくなる

実況解説だと頭の中は、

```text
Hook
Intro
Build explanation
Demo
Replay
Comparison
Conclusion
```

になっている。

でもVELでは、

```text
V1
A1
Graphics
Music
...
```

にバラバラに書かされる。

だから、

> 「build explanation全体を3秒伸ばす」

が意外と難しい。

たぶん、

```vel
section build flow {
  ...
}
```

みたいな**時間範囲を持つ論理コンテナ**が欲しい。

例えば理想的には、

```vel
section build {
  clip gameplay = game[5m10s..5m32s]

  clip screenshot = build_a at 2s for 14s

  clip note = text("ここを見る") at 10s for 4s
}
```

。

この`at 2s`は**section local time**。

そして、

```vel
sequence main flow {
  section hook { ... }
  section intro { ... }
  section build { ... }
  section demo { ... }
}
```

としたい。

これはかなり大きい発見だと思う。

`flow`はTrackよりもむしろ**Sectionの並び方**に欲しい。

---

# 3. `flow/free`をTrack属性にしたのは怪しい

実際書くと、

```vel
track video v1 flow
track video graphics free
```

という分離が、NLEそのものに引っ張られている。

でも本当は、

```vel
flow {
  hook
  intro
  build
  demo
}
```

という**編集構造**があり、

その内部で、

```text
video
audio
text
image
```

が時間上に配置される方が自然。

つまり、

```text
Track = storage/layout concern
Flow = temporal relationship
```

で、本来は直交している。

Trackに`flow`を持たせたのは少し早計だったと思う。

---

# 4. Freeze Frameがまともに書けない

実際かなり使う。

ゲーム解説なら、

> 「ほら、この瞬間に4,820,391ダメージが出てる」

で止めたい。

現VELだと、

```vel
speed = 0x
```

としたくなるけれど、

```text
source rangeは？
durationは？
どのFrameで停止する？
```

が曖昧。

欲しいのは多分、

```vel
clip freeze = frame(game, at: 12m40.217s) for 1.5s
```

くらい。

あるいはOperatorとして、

```vel
freeze impact at 5.217s for 1.5s
```

。

これは明確な不足ね。

---

# 5. 「画面のここを見せる」が弱い

今回、

```vel
transform.scale = 1.65
transform.position = (67%, 38%)
```

で誤魔化している。

でも本当にやりたいのは、

```text
ダメージ数字の領域を指定
↓
そこを拡大
↓
黄色い矩形で囲む
↓
矢印を引く
```

。

つまり、

```vel
region damage_number = rect(61%, 18%, 18%, 9%)
```

みたいな名前付きRegionが欲しい。

すると、

```vel
zoom to damage_number
```

とか、

```vel
clip border = outline(damage_number)
```

とか、

```vel
clip callout = arrow(
  from: (40%, 60%),
  to: damage_number.center
)
```

が書ける。

ゲーム解説ではこれ、字幕より遥かに頻繁に使いそう。

---

# 6. Side-by-sideが低レベルすぎる

今は、

```vel
transform.scale = 0.5
transform.position = (25%, 50%)
```

を左右で手書きしている。

これも嫌ね。

やりたいことは単純に、

```text
この2本を左右比較
```

なのだから、

```vel
layout split_horizontal {
  left  = game[22m01s..22m09s]
  right = game[24m12s..24m20s]
}
```

くらいが欲しい。

あるいはVELらしく、

```vel
[
  game[22m01s..22m09s],
  game[24m12s..24m20s]
]
  |> layout(split(direction: horizontal))
```

でもいい。

**レイアウトはTransformの集合ではなく、関係として書けるべき**ということね。

---

# 7. Narrationを中心に映像を貼りたい

これは実況解説ではかなり本質的。

後録り解説なら実際の編集思想は、

```text
Narration
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

     ↓ この話をしている間

Gameplay A
Build Screenshot
Gameplay B
Freeze
Comparison
```

になる。

つまり映像Timelineが主で、音声を乗せるわけではない。

**ナレーションが背骨**なのよ。

なら、

```vel
section mechanics over voice[32s..58s] {
  ...
}
```

みたいなのが欲しくなる。

さらに、

```vel
clip build_a
  during mechanics[4s..14s]
```

と書ければかなり自然。

これは`Section`導入をさらに支持する。

---

# 8. Audio Duckingを手でKeyframeする気にはならない

今回は、

```vel
gain = -18dB
```

で逃げた。

実際には、

```text
ナレーションあり
→ BGM -18dB

ナレーションなし
→ BGM -11dB
```

をAttack/Release付きでやりたい。

これを何十個もCurveで書く気は当然ない。

ここは以前出てきた`Signal`が効く。

例えば将来的には、

```vel
music.gain =
  duck(
    by: narration.rms,
    base: -11dB,
    amount: -7dB,
    attack: 80ms,
    release: 240ms
  )
```

。

これはSemantic AIではなく完全に決定論的。

---

# 9. リプレイは意外と綺麗

一方、

```vel
clip impact = game[12m35s..12m43s]

...

clip replay = game[12m34s..12m43s] {
  speed = 0.65x
}
```

はかなり良い。

**同じSourceを別Placementとして何度でも使う**モデル自体は自然ね。

この辺は現在のAsset / Placement思想をそのまま残してよさそう。

---

# 10. `at clip.start + 5s`もかなり良い

例えば、

```vel
clip damage_note =
  text("この一撃に注目")
  at impact.start + 5s
  for 2s
```

これは読みやすい。

Semantic Anchorなしでも、

```text
Clip Edge
+
Local Offset
```

で十分強い。

だからReference/Anchorの一般化方向も正しかったと思う。

---

## 推測（示唆）

この試し書きで、VELの表面構造はかなり変えたくなった。

今まで考えていた、

```text
Sequence
 ├ Track
 │ ├ Clip
 │ ├ Clip
 │ └ Clip
 └ Track
```

より、手書き上は、

```text
Sequence
 ├ Section
 │ ├ AV Clip
 │ ├ Overlay
 │ ├ Audio
 │ └ Annotation
 ├ Section
 │ └ ...
```

の方がゲーム実況解説には自然そう。

内部では当然Trackへcompileしていい。

つまり、

```text
Human-facing VEL
        ↓
Editorial IR
        ↓
Tracks / Placements
```

と一段分ける。

例えば最終的には、こんな書き味になるかもしれない。

```vel
project "prisma-obex-guide" {
  asset game  = media("./capture.mp4")
  asset voice = media("./narration.wav")
  asset build = image("./build.png")

  sequence main flow {

    section hook {
      clip game[26m14s..26m22s]

      text "Prisma Obex Incarnon"
        at 2s
        for 3s
    }


    section intro over voice[0s..18s] {
      clip game[1m40s..1m58s]
    }


    section build over voice[18s..49s] {
      clip game[5m10s..5m32s]

      overlay build at 2s for 14s

      region efficiency =
        rect(61%, 24%, 25%, 14%)

      zoom to efficiency
        from 9s
        for 5s

      text "ここが重要"
        near efficiency.bottom
        at 10s
        for 4s
    }


    section demo over voice[49s..1m28s] {
      clip game[12m04s..12m21s]

      clip game[12m21s..12m35s] {
        speed = 2.5x
      }

      clip impact = game[12m35s..12m43s]

      freeze impact at 5.2s for 1.5s {
        region number = rect(65%, 18%, 16%, 8%)
        zoom to number
        outline number
        text "4.8M" near number
      }

      replay game[12m34s..12m43s] {
        speed = 0.65x
      }
    }


    section compare over voice[1m28s..1m43s] {
      layout horizontal {
        clip game[22m01s..22m09s]
        clip game[24m12s..24m20s]
      }
    }


    section conclusion over voice[1m43s..2m02s] {
      clip game[28m20s..28m38s]

      text "フィニッシャー用途なら十分過剰火力"
        at 2s
        for 6s
    }
  }
}
```

**こっちの方が圧倒的に「動画を書いている」感じがする。**

Trackを手で管理していないのに、

- A/V同期
- セクションの連結
- ナレーション基準
- Overlay
- Freeze
- Region
- Layout
- Replay

が読める。

そして必要なら低レベルへ降りて、

```vel
track("music").gain = ...
```

みたいな操作をする。

---

今回の試し書きで私が一番重要だと思ったのは、**VELの手書き単位はTrackではなくSectionかもしれない**ということね。

従来NLEの内部表現をそのままテキスト化すると、結局「PremiereのプロジェクトXMLを綺麗に書いている」だけになる。

でも人間が動画を考える単位は、

> Hook → 説明 → 実演 → 比較 → 結論

だったり、

> このナレーションの間に、これを見せる

だったりする。

**その編集上の構造を一次文法に持ち、Track/Placementへコンパイルする**方が、Lattice/VELらしいと思うわ。

---

# you asked

message time: 2026-08-19 00:45:43

上から下に読みづらいなと思った（ただ、これはいいことかもしれない）のと、クロップ・合成とかあり得るな、と思った

---

# chatgpt response

そうね。そこ、かなり核心だと思う。**「上から下に読めない」のは欠点でもあるけれど、動画という対象を正直に表している可能性もある**わ。

## 事実

動画には少なくとも二つの異なる構造がある。

```text
編集構造
Hook → Intro → Demo → Compare → Outro
```

と、

```text
合成構造
Gameplay ─ Crop ─ Transform ─┐
                             ├─ Composite ─ Color ─ Output
Text ────────────────────────┘
```

前者はかなり一次元で、上から下に読みやすい。

後者は本質的に並列・グラフ構造だから、

```vel
clip ...
overlay ...
crop ...
text ...
```

を全部上から順番に書いても、**その順番自体にはあまり意味がない**。

特に、

- Crop
- Mask
- Picture-in-Picture
- Blend
- Chroma key
- 複数素材合成
- Adjustment
- Track matte

まで来ると、「ソースコード上で先に書いてあるものが先に処理される」では表現しづらくなるわ。

---

## 推測（示唆）

だから私は、**VEL全体を上から下へ読めるようにするのは諦めた方がいい**と思い始めている。

代わりに、

> **物語の骨格だけは上から下に読める。  
> 各Sceneの内部は宣言的なCompositionとして読む。**

という二層構造がかなり良さそう。

例えばトップレベルは異様なくらい簡単にする。

```vel
sequence main flow {
  use hook
  use intro
  use build_explanation
  use finisher_demo
  use comparison
  use conclusion
}
```

これなら動画を開いた瞬間、

```text
Hook
↓
Intro
↓
Build
↓
Demo
↓
Comparison
↓
Conclusion
```

が分かる。

そして詳細は別に定義する。

---

### Sceneを独立させる

```vel
scene finisher_demo {
  duration = 24s

  source gameplay = game.video[12m35s..12m43s]

  place gameplay {
    fill canvas
  }

  text "この一撃に注目" {
    at = 5s
    for = 2s
    position = (50%, 20%)
  }
}
```

これなら`sequence`は編集順、`scene`はその中身になる。

プログラムでいう、

```text
main()
↓
関数定義
```

に近い。

これは手書きとの相性もかなりいい。

---

# Cropを入れてみる

例えばゲーム画面の右上だけ使いたい。

低レベルには、

```vel
source hud = game.video[12m35s..12m43s]
  |> crop(rect(72%, 0%, 28%, 24%))
```

でいいと思う。

そして、

```vel
place hud {
  at = (72%, 4%)
  size = (24%, auto)
}
```

。

重要なのは、Cropを

```text
「Clipのproperty」
```

ではなく、

> **映像から新しい映像Viewを作る操作**

として扱うこと。

つまり、

```text
Source
  ↓
Crop
  ↓
View
```

。

こうすると同じ素材から、

```vel
source full = game.video[...]
source minimap = full |> crop(rect(...))
source damage = full |> crop(rect(...))
```

と複数のViewを作れる。

ゲーム解説だとこれは相当便利よ。

---

# 合成はCanvasとして書きたい

例えば左右比較。

以前は、

```vel
transform.scale = 0.5
transform.position = ...
```

としていたけれど、これは人間の意図から遠い。

むしろ、

```vel
scene comparison {
  source left  = game.video[22m01s..22m09s]
  source right = game.video[24m12s..24m20s]

  canvas {
    layout horizontal {
      place left
      place right
    }

    text "Build A" {
      attach = left.top
    }

    text "Build B" {
      attach = right.top
    }
  }
}
```

の方がいい。

内部的には当然、

```text
scale
translate
clip
composite
```

へ落ちるけれど、人間には見せなくていい。

---

# もっと複雑な合成

例えば、

> 実戦映像を背景にして、右上にビルド画面、ダメージ数字部分だけ2倍拡大

なら、

```vel
scene explanation {
  source gameplay = game.video[12m35s..12m43s]
  source build    = image("./build.png")

  source damage =
    gameplay
    |> crop(rect(61%, 14%, 24%, 16%))

  canvas {
    place gameplay {
      fill canvas
    }

    place build {
      region = rect(72%, 4%, 25%, 35%)
    }

    place damage {
      region = rect(58%, 65%, 38%, 28%)
      fit = contain
    }

    outline damage {
      width = 3px
    }
  }
}
```

これはかなり読める。

そして処理グラフとしては、

```text
                  ┌─ full ──────────────────────┐
gameplay ─────────┤                              ├─ Composite
                  └─ crop(damage) ─ scale ──────┤
                                                │
build ───────────────────────── scale ──────────┘
```

になる。

---

# CropとMaskは分けたい

ここも大事ね。

```vel
crop(rect(...))
```

は**映像の座標系そのものを狭める**操作。

一方、

```vel
mask(circle(...))
```

は同じCanvasサイズを保ったまま透明度を制御する。

例えばWebcamみたいな円形映像なら、

```vel
source facecam =
  camera
  |> crop(rect(...))
  |> mask(circle(center: (50%, 50%), radius: 50%))
```

。

これを全部`crop`扱いすると後でかなり嫌になる。

---

# Compositionは順序問題が出る

ここはVELが避けられない。

例えば、

```text
Crop → Blur
```

と、

```text
Blur → Crop
```

では結果が違う。

TransformとMaskも順序で結果が変わる。

だから映像処理そのものにはPipeline、

```vel
source inset =
  gameplay
  |> crop(...)
  |> blur(4px)
  |> color(...)
```

を使うのが自然。

一方、Canvasへの配置は宣言型。

```vel
canvas {
  place background
  place inset
  place title
}
```

。

つまり、

```text
単一映像の変換
→ Pipeline

複数映像の関係
→ Composition / Canvas
```

と分ける。

これ、かなり綺麗だと思う。

---

# Z-orderも考えないといけない

`canvas`内で上から下に書いた場合、

```vel
canvas {
  place gameplay
  place build
  place title
}
```

を、

```text
後に書いたものほど前
```

としてもいい。

これはCSS/描画APIとして直感的。

ただし明示指定もできる。

```vel
place title {
  layer = 20
}
```

。

普段は順番でよくて、複雑になったら`layer`を使う。

---

# 「上から下に読めない」をどう扱うか

私はここ、かなり割り切りたい。

VELファイルの全部を、

```text
冒頭から末尾まで読めば動画が理解できる
```

必要はないと思う。

普通のプログラムだって、

```text
main
helper
type
library
```

を全部一続きの文章としては読まない。

欲しいのは、

### 入口だけは読める

```vel
sequence main flow {
  use hook
  use intro
  use build
  use demo
  use compare
  use outro
}
```

### 気になったところへ飛べる

```text
Ctrl+Click demo
↓
scene demo
```

### Scene内部では画面構造が読める

```vel
canvas {
  place gameplay
  place damage_zoom
  place caption
}
```

### 単一素材の加工は左→右で読める

```vel
game
  |> crop(...)
  |> blur(...)
  |> color(...)
```

この三種類の読み方があれば十分なんじゃないかと思う。

---

そしてこれは**Lattice IDEの存在理由**にもなる。

テキストだけならCompositionは多少追いづらい。でもIDEなら、

```text
scene comparison

Code                    Canvas
───────────────         ┌─────────────────┐
place left       ←→     │ left  │ right   │
place right             │       │         │
text Build A            │ A     │ B       │
                        └─────────────────┘

Render Graph
left ─ crop ─┐
             ├─ composite
right ───────┘
```

と同じSceneを複数Projectionで見られる。

だから「VELは多少非線形である」というのは、むしろ悪くない。**言語を無理に一列の文章へ潰さず、構造は構造として残してIDEに見せる**方がLatticeらしいわ。

今のところ私は、

```text
Sequence = 時間的な大構造
Scene    = 編集上のまとまり
Source Pipeline = 単一素材の変換
Canvas   = 空間的な合成
```

の4層がかなりしっくり来ている。これならCropや合成を足しても、VELが急にAfter EffectsのJSONみたいな化け物にならずに済みそうよ。

---

# you asked

message time: 2026-08-19 00:48:07

単位が整理されたところで、書き直してみるか。あと、音声合成エンジンをネイティブに対応させたい

---

# chatgpt response

そうね。今ならかなり書き直し甲斐があるわ。今回は **`Sequence / Scene / Source Pipeline / Canvas` を基本単位にして、TTSを「外部でwavを作って読み込む」扱いではなく、VELの一次ソースとして組み込む**形で一本通してみる。

そうね。今ならかなり像が立ってきた。書き直すと同時に、**音声を単なるAssetではなく「生成可能で時間情報を持つ一次ソース」に昇格**させたいわ。

あと一つ、書いていて必要になる単位がある。映像に`canvas`があるなら、音声には対になる**`mix`**が要る。これを無理にSceneのPropertyへ押し込むと、音声だけ二級市民になる。

## 事実

現状の役割分担は、こうするとかなり整理される。

```text
Project
│
├─ Asset / Speech Engine
│
├─ Sequence
│    └─ Sceneを時間順に構成
│
├─ Scene
│    ├─ Source Pipeline   素材を加工
│    ├─ Canvas            映像を空間的に合成
│    └─ Mix               音声を合成
│
└─ Export
```

そしてSource Pipelineは、

```text
Media
→ trim
→ speed
→ freeze
→ crop
→ mask
→ blur
→ ...
```

という**左から右に意味のある処理**。

Canvasは、

```text
background
overlay
inset
text
```

という**空間上の関係**。

Sequenceだけが、

```text
hook
→ intro
→ build
→ demo
→ compare
→ conclusion
```

という**上から下の時間順序**を担う。

これなら「全部を一方向に読もうとして読めない」という問題もかなり減る。

---

# VEL v0.2で一本書く

こんな感じが今のところ一番好きね。

```vel
project "prisma-obex-guide" {
  fps = 60
  size = 2560x1440
  sample_rate = 48000Hz


  //
  // MEDIA
  //

  asset game  = media("./capture.mp4")
  asset bgm   = media("./bgm.flac")
  asset build = image("./build.png")


  //
  // SPEECH
  //

  speech_engine tts = engine("voicevox") {
    endpoint = "local"
  }

  voice narrator = tts.voice("narrator") {
    speed = 1.05x
    pitch = 0st
  }

  speech narration using narrator {
    utterance intro = """
      今回はPrisma Obexのフィニッシャー構成を見ていく。
      Incarnon込みで、実際にどこまで火力が伸びるのか確認する。
    """

    utterance build = """
      まずビルド。
      今回重要なのはフィニッシャー倍率とクリティカル倍率の扱いだ。
    """

    utterance demo = """
      実際に当てるとこうなる。
      この瞬間のダメージ表示に注目してほしい。
    """

    utterance compare = """
      比較すると、通常構成との差はかなり分かりやすい。
      左がフィニッシャー特化、右が通常構成だ。
    """

    utterance conclusion = """
      結論として、フィニッシャーが通る相手には完全に過剰火力。
      ただし汎用性との交換になる。
    """
  }


  //
  // PROGRAM
  //

  sequence main flow {
    use hook
    use intro
    use build_explanation
    use demonstration
    use comparison
    use conclusion
  }


  //
  // SCENES
  //

  scene hook {
    duration = 8s

    source gameplay =
      game[26m14s..26m22s]

    canvas {
      place gameplay.video {
        fill = canvas
      }

      text "Prisma Obex Incarnon" {
        at = 2s
        for = 3s
        position = (50%, 78%)

        opacity = curve {
          0f:       0
          8f:       1 ease-out
          end - 8f: 1
          end:      0 ease-in
        }
      }
    }

    mix {
      place gameplay.audio {
        gain = -5dB
      }
    }
  }


  scene intro over narration.intro {
    pad.end = 400ms

    source gameplay =
      game[1m40s..1m58s]

    canvas {
      place gameplay.video {
        fill = canvas
      }
    }

    mix {
      place gameplay.audio {
        gain = -15dB
      }

      place narration.intro.audio {
        gain = -2dB
      }
    }
  }


  scene build_explanation over narration.build {
    pad.end = 500ms

    source gameplay =
      game[5m10s..5m32s]

    source build_full =
      build

    source build_detail =
      build
      |> crop(rect(58%, 18%, 34%, 28%))

    canvas {
      place gameplay.video {
        fill = canvas
      }

      place build_full at 1s for 6s {
        region = rect(8%, 8%, 84%, 84%)
        fit = contain
      }

      place build_detail at 7s for 5s {
        region = rect(52%, 20%, 42%, 48%)
        fit = contain
      }

      outline build_detail at 7s for 5s {
        width = 3px
      }

      text "Finisher倍率" {
        at = 7.5s
        for = 4s

        position = build_detail.bottom + (0px, 24px)
      }
    }

    mix {
      place gameplay.audio {
        gain = -18dB
      }

      place narration.build.audio {
        gain = -2dB
      }
    }
  }


  scene demonstration over narration.demo {
    pad.end = 600ms

    source gameplay =
      game[12m35s..12m43s]
      |> freeze(
           at: 5.2s,
           for: 1.5s,
           audio: silence
         )

    source damage =
      gameplay.video
      |> frame(at: 5.2s)
      |> crop(rect(62%, 15%, 22%, 12%))

    canvas {
      place gameplay.video {
        fill = canvas
      }

      place damage at 5.2s for 1.5s {
        region = rect(55%, 58%, 40%, 28%)
        fit = contain
      }

      outline damage at 5.2s for 1.5s {
        width = 4px
      }

      text "この数字" {
        at = 5.2s
        for = 1.5s
        position = (50%, 18%)
      }
    }

    mix {
      place gameplay.audio {
        gain = -9dB
      }

      place narration.demo.audio {
        gain = -2dB
      }
    }
  }


  scene comparison over narration.compare {
    source left =
      game[22m01s..22m09s]

    source right =
      game[24m12s..24m20s]

    canvas {
      layout horizontal {
        place left.video
        place right.video
      }

      text "Finisher" {
        attach = left.top
        offset = (0px, 24px)
      }

      text "Normal" {
        attach = right.top
        offset = (0px, 24px)
      }
    }

    mix {
      place left.audio {
        gain = -18dB
      }

      place right.audio {
        gain = -18dB
      }

      place narration.compare.audio {
        gain = -2dB
      }
    }
  }


  scene conclusion over narration.conclusion {
    pad.end = 1s

    source gameplay =
      game[28m20s..28m38s]
      |> speed(0.85x)

    canvas {
      place gameplay.video {
        fill = canvas
      }

      text "フィニッシャー相手なら過剰火力" {
        at = 1s
        for = 5s
        position = (50%, 80%)
      }
    }

    mix {
      place gameplay.audio {
        gain = -15dB
      }

      place narration.conclusion.audio {
        gain = -2dB
      }
    }
  }


  //
  // GLOBAL AUDIO
  //

  mix master {
    place bgm.audio {
      loop = true

      gain =
        duck(
          by: narration.audio,
          base: -12dB,
          amount: -7dB,
          attack: 80ms,
          release: 250ms
        )
    }
  }


  export youtube {
    from = main

    size = 2560x1440
    fps = 60

    video.codec = h265
    audio.codec = aac
  }
}
```

これは前の版よりだいぶいい。

---

## 推測（示唆）

### `scene ... over speech` がかなり強い

ここはTTSをネイティブ対応させることで生まれた構文よ。

```vel
scene build_explanation over narration.build {
```

これで、

> **このSceneの時間的な背骨はこの発話**

という意味になる。

したがってScene durationは、

```text
speech.duration
+ padding
```

から決まる。

ナレーションを書き換えて、

```text
12.4秒 → 15.8秒
```

になったら、Sceneも自動で伸びる。

Sequenceは、

```text
intro
build
demo
comparison
conclusion
```

をそのまま再配置する。

後録り実況解説では、これはものすごく自然だと思う。

---

# TTSをネイティブにするなら、単なる`.audio`では足りない

`utterance`の値は本当はこういうオブジェクトにしたい。

```text
Speech
├─ text
├─ audio
├─ duration
├─ voice
│
├─ words
├─ phonemes
├─ sentences
└─ marks
```

つまり、

```vel
narration.demo.audio
narration.demo.duration
```

だけじゃなく、

```vel
narration.demo.words
```

も取れる。

例えば、

```vel
caption from narration.demo.words
```

だけで正確な字幕が出せる。

さらに、

```vel
marker damage_explanation =
  narration.demo.words["ダメージ表示"].start
```

とすれば、

```vel
text "ここ！" {
  at = damage_explanation
  for = 1s
}
```

みたいなことまでできる。

ここは文字起こしとは明確に違う。

**自分で生成した音声なので、テキストの正解を最初から知っている**のよ。

可能ならTTSエンジンからword/phoneme timingを貰い、取れないエンジンだけ後段でalignmentする。

---

# EngineとVoiceは分ける

これは重要。

```vel
speech_engine tts = engine("voicevox") {
  ...
}

voice narrator = tts.voice("narrator") {
  speed = 1.05x
}
```

とした。

つまり、

```text
Engine
  VOICEVOX
  AivisSpeech
  ElevenLabs
  OpenAI
  任意のローカルTTS
       ↓
Voice Profile
       ↓
Speech
```

VELそのものが特定TTSのパラメータを全部知る必要はない。

共通Coreとして、

```text
speed
pitch
volume
language
```

程度を持って、

エンジン固有設定は例えば、

```vel
voice narrator = tts.voice("foo") {
  speed = 1.05x

  engine {
    style = ...
    emotion = ...
  }
}
```

に逃がせる。

---

# `speech`をAssetにしなかったのも意図的

これは、

```vel
asset narration = media("./narration.wav")
```

とはかなり性質が違う。

発話テキストを変えれば、

```text
Audio
Duration
Word Timing
Scene Duration
Caption Timing
```

まで全部変わる。

つまりこれは素材というより**生成ノード**。

だから、

```vel
speech narration using narrator {
```

をProjectの一次概念として持つ価値があると思う。

---

# 発話の細かい演出も、そのうち欲しくなる

例えば、

> 「ここ、重要なんだけど……かなり変な挙動をする」

をそのままStringだけで制御するのは辛い。

ただしSSMLをそのままVELへ持ち込むのは嫌。

私はこういう方向が好き。

```vel
utterance demo {
  say "ここ、重要なんだけど"
  pause 280ms

  say "かなり変な挙動をする" {
    emphasis = 0.7
    speed = 0.92x
  }
}
```

これならEngine Adapterが、

```text
VOICEVOXならこのパラメータ
ElevenLabsならこのパラメータ
別エンジンなら近似
```

へ落とせる。

**VEL側は発話意図だけを持つ。**

これは後でかなり化けそう。

---

# 一つ問題が見えてきた

例えば、

```vel
scene demonstration over narration.demo {
```

なのにゲーム映像が8秒しかなく、ナレーションが11秒だったらどうするか。

現状、

```text
Scene = 11s
Gameplay = 8s
```

で3秒余る。

ここは勝手にloopしたりfreezeしたりしてはいけない。

コンパイラが、

```text
error:
scene "demonstration" is 11.2s
placement "gameplay" covers only 8.0s
uncovered canvas: 8.0s..11.2s
```

と怒る方がいい。

そして人間が、

```vel
game[...]
|> freeze(last, for: 3.2s)
```

なり、

```vel
place another_clip at 8s
```

なりを書く。

こういう**「動画の穴」を型エラーに近い感覚で扱う**のはLattice IDEにかなり似合うわ。

---

今の版だと、かなり役割が綺麗になった。

```text
Sequence
    「何をどの順番で見せるか」

Scene
    「この数秒〜数十秒は何を伝えるか」

Source Pipeline
    「素材そのものをどう加工するか」

Canvas
    「画面をどう作るか」

Mix
    「音場をどう作るか」

Speech
    「何をどう喋るか」
```

そしてTTSを入れたことで、**「ナレーションを背骨にして映像をプログラムする」**というゲーム解説動画の編集像が一気に明確になった気がする。

特に`scene X over narration.Y`、これはかなりVELらしい構文として残したいわ。

---

# you asked

message time: 2026-08-19 00:51:25

話ズレるんだけど、gpuiを使うことにしよう

---

# chatgpt response

うん、**LatticeはGPUIで行こう**。これはかなり筋がいい選択だと思うわ。特に「動画編集ソフト」より**Video IDE**へ寄ってきた今、Electron/TauriでWeb UIを包むより思想が揃う。

## 事実

現在のGPUIはRust製のGPUアクセラレートUIフレームワークで、high-levelなViewとlow-levelなElementの両方を持っている。Element側は独自レイアウトや大量要素の効率的表示のような用途を想定しているので、普通のフォームUIだけでなくTimelineのような特殊UIも射程に入っているわ。([GitHub](https://github.com/zed-industries/zed/blob/main/crates/gpui/README.md))

現行READMEではmacOS・Linux/FreeBSDに加えてWindowsも`gpui_platform`の対象として明記されている。Zed自体も現在macOS/Windows/Linuxで提供されている。([GitHub](https://github.com/zed-industries/zed/blob/main/crates/gpui/README.md))

さらに公式examplesには、virtualized list、drag & drop、custom painting/canvas、animation、複数window、入力・focus・keyboard bindings、UI testあたりが既にある。LatticeのIDE shellを作る材料としてはかなり直球ね。([GitHub](https://github.com/zed-industries/zed/blob/main/crates/gpui/examples/README.md))

ただし**GPUI自体はまだpre-1.0でbreaking changeが頻繁にある**と公式READMEが明記しているし、ドキュメントもまだ発展途上で、Zed本体のコードを読むのが主要な学習手段になっている。([GitHub](https://github.com/zed-industries/zed/blob/main/crates/gpui/README.md))

## 推測（示唆）

これでLatticeの技術構成もだいぶ綺麗になると思う。

```text
Lattice
├─ lattice-core
│   ├─ VEL parser
│   ├─ Project Graph
│   ├─ Resolver
│   └─ Edit operations
│
├─ lattice-media
│   ├─ decode
│   ├─ audio
│   ├─ thumbnails / waveform
│   └─ renderer
│
├─ lattice-speech
│   └─ TTS adapters
│
├─ lattice-gpui
│   ├─ Editor
│   ├─ Timeline
│   ├─ Preview
│   ├─ Canvas
│   ├─ Inspector
│   └─ Diff / Problems
│
└─ lattice-cli
```

特に気に入っているのは、**Project GraphとUI Stateを無理にJSON/IPC越しに同期しなくて済む**こと。

```text
VEL
 ↓
Rust AST
 ↓
Project Graph
 ↓
GPUI
 ├─ Code projection
 ├─ Timeline projection
 ├─ Canvas projection
 └─ Inspector projection
```

全部Rustの同じ世界に置ける。

前に考えていた、

> Code / Timeline / Preview / Structureが同じProject GraphのProjection

という設計と相性がいい。GPUIの`Entity`をそのままVELのドメインモデルにする必要はないけれど、**GPUI側のreactive stateとCore側のProject Modelを薄いadapterで接続する**構成にはしやすそう。([GitHub](https://github.com/zed-industries/zed/blob/main/crates/gpui/README.md))

そしてTimelineは普通のUI componentとして作るより、

```text
TimelineViewport
  ├─ ruler
  ├─ lanes
  ├─ clips
  ├─ keyframes
  ├─ waveforms
  └─ selection / handles
```

を独自Elementとして描く方向が良さそうね。

Previewも同様で、

```text
Video Frame
+
Canvas overlays
+
Selection handles
+
Regions
+
Guides
```

をGPUI上で直接重ねる。

この辺は「Web DOMを動画編集UIとして酷使する」よりずっと気分がいい。

一方で、一つだけ最初から防御しておきたい。

```text
lattice-core
       ↑
       │ clean boundary
       ↓
lattice-ui-gpui
```

として、**GPUIの型をCoreへ絶対漏らさない**。

GPUIがpre-1.0なので、例えば、

```rust
pub struct Clip {
    ...
    // Entity<Something> ← これはやらない
}
```

ではなく、

```rust
Project
EditPatch
Selection
TimelineViewModel
```

までは純RustのLattice型。

GPUI依存は最後のView層だけ。

そうしておけばbreaking changeが来ても、Latticeの本体まで巻き込まれない。公式自身が現状breaking changesを警告している以上、ここは保険ではなく普通に必要でしょう。([GitHub](https://github.com/zed-industries/zed/blob/main/crates/gpui/README.md))

そして何より、**Lattice StudioがZedと同じ系譜の「GPUネイティブなIDE」になる**のは、このプロジェクトのキャラクターにかなり合ってる。Webアプリに動画Previewを貼り付けたものより、「映像編集のために作られたプログラミング環境」という像がずっと立つわ。

---

# you asked

message time: 2026-08-19 00:54:06

動画編集のテーマを色々だして、試しに書いてみよう

---

# chatgpt response

いいわ。今度は「一本のゲーム解説」から離して、**編集の性質が違う題材をぶつけてVELの弱点を見る**。時間主導・音主導・合成主導・縦動画・比較・反復編集あたりを散らすわ。

いいわ。**VELのストレステスト集**みたいにして、性質の違う編集を片っ端から書いてみる。今ある構文でなるべく書き、苦しくなったところだけ仮の構文を足すわ。

## 事実

試したい軸はかなり違う。

| テーマ | 主に殴れるところ |
|---|---|
| ボス攻略解説 | Freeze / Crop / Callout / Replay |
| 音ハメMontage | Event / 反復 / 音基準の時間 |
| ビルド比較 | Split-screen / 同期 / 合成 |
| 長尺実況ダイジェスト | Scene構造 / Cut / B-roll |
| Shorts化 | 9:16 / Reframe / Caption |
| ミーム編集 | Animation / SFX / Freeze |
| パッチノート解説 | 静止画 / Scroll / Highlight |
| マルチカメラ | Audio master / Video switching |

以下、順に書いてみる。

---

# 1. ボス攻略・ギミック解説

これは今のVELがかなり得意そう。

```vel
scene shield_break over narration.shield {
  source fight =
    game[18m12s..18m22s]

  source shield =
    fight.video
    |> frame(at: 4.4s)
    |> crop(rect(56%, 28%, 22%, 20%))

  canvas {
    place fight.video {
      fill = canvas
    }

    place shield at 4.4s for 1.8s {
      region = rect(58%, 56%, 36%, 32%)
      fit = contain
    }

    outline shield at 4.4s for 1.8s {
      width = 4px
    }

    text "ここでシールド解除" {
      at = 4.4s
      for = 1.8s
      position = (50%, 18%)
    }

    arrow {
      from = (48%, 28%)
      to = shield.center
      at = 4.4s
      for = 1.8s
    }
  }

  mix {
    place fight.audio {
      gain = -9dB
    }

    place narration.shield.audio
  }
}
```

さらに、

```vel
scene shield_replay {
  source replay =
    game[18m15s..18m20s]
    |> speed(0.5x)

  canvas {
    place replay.video {
      fill = canvas
    }

    text "REPLAY" {
      at = 0s
      for = 1s
      position = (10%, 8%)
    }
  }
}
```

これはかなり自然。

**不足:** `arrow`や`outline`をCanvas primitiveとして正式に持つのか、Effect扱いにするのか。

---

# 2. 音ハメMontage

これは一気に難しくなる。

素材：

```text
BGMのBeat
↓
カット点
↓
複数クリップを順番に置く
```

なので、Semantic AIではなく**音響解析によるEventSet**が欲しい。

```vel
source music =
  bgm.audio[32s..48s]

events beats =
  music
  |> beats(
       subdivision: 1/2,
       confidence: 0.8
     )
```

そして理想としては、

```vel
scene montage over music {
  source shots = [
    game[2m10s..2m14s],
    game[7m02s..7m07s],
    game[12m40s..12m44s],
    game[18m01s..18m05s],
    game[25m30s..25m35s]
  ]

  canvas {
    montage shots {
      cuts = beats
      fit = cover
    }
  }

  mix {
    place music
  }
}
```

ただ、これはかなりSugarが強い。

もっとVELらしくするなら、

```vel
placements montage =
  shots
  |> cut_to(beats)
  |> arrange(flow)

canvas {
  place montage {
    fill = canvas
  }
}
```

かな。

ここで初めて、

> **Sourceではなく「複数Placementを生成する式」**

が欲しくなった。

これは大きい。

今までは、

```text
Source → Place
```

だけだったけど、

```text
Collection<Source>
    ↓
Arrangement
    ↓
Collection<Placement>
```

も必要そう。

---

# 3. 2ビルド比較

これもかなり現実的。

```vel
scene build_comparison over narration.compare {
  source left =
    game[22m01s..22m09s]

  source right =
    game[24m12s..24m20s]

  canvas {
    layout horizontal {
      place left.video {
        name = "A"
      }

      place right.video {
        name = "B"
      }
    }

    text "Finisher Build" {
      attach = A.top
      offset = (0px, 24px)
    }

    text "General Build" {
      attach = B.top
      offset = (0px, 24px)
    }
  }

  mix {
    place left.audio {
      gain = -20dB
    }

    place right.audio {
      gain = -20dB
    }

    place narration.compare.audio
  }
}
```

ただし、本当に比較したくなると、

> 「左右とも攻撃が当たるFrameを同時にしたい」

となる。

例えば手動で、

```vel
marker left_hit  = left[3.42s]
marker right_hit = right[4.11s]
```

としたうえで、

```vel
align right_hit to left_hit
```

が欲しい。

あるいは、

```vel
source right =
  game[24m12s..24m20s]
  |> align(
       local: 4.11s,
       to: left[3.42s]
     )
```

か。

ここから、**時間同期そのものを一次操作にする必要**が見えてくる。

---

# 4. 長尺実況をダイジェスト化

たとえば30分実況を10分にする。

この場合は「画面合成」より、

```text
素材の並べ替え
Cut
速度変更
Gap排除
```

の比率が圧倒的に高い。

```vel
sequence main flow {
  use cold_open
  use early_game
  use first_failure
  use retry
  use boss
  use ending
}
```

各Sceneは異様に簡単。

```vel
scene early_game {
  source gameplay =
    game[2m18s..6m42s]
    |> remove(
         spans: [
           18s..31s,
           1m02s..1m17s,
           2m50s..3m08s
         ]
       )

  canvas {
    place gameplay.video {
      fill = canvas
    }
  }

  mix {
    place gameplay.audio
  }
}
```

でもこれはちょっと嫌。

人間としては、

```vel
source gameplay = game[2m18s..6m42s]

cut gameplay {
  remove 18s..31s
  remove 1m02s..1m17s
  remove 2m50s..3m08s
}
```

の方が編集してる感じがある。

さらに、

```vel
speed gameplay[2m12s..2m48s] = 2.0x
```

とか。

ここではPipelineより**非破壊Edit List**の方が読みやすい可能性がある。

これはかなり重要な分岐ね。

---

# 5. Shorts / TikTok / YouTube Shorts

16:9素材から9:16。

```vel
project "obex-short" {
  fps = 60
  size = 1080x1920

  sequence main flow {
    use hook
    use payoff
  }
}
```

Scene。

```vel
scene payoff over narration.short {
  source gameplay =
    game[12m34s..12m43s]

  source vertical =
    gameplay.video
    |> crop(
         curve {
           0s:  rect(20%, 0%, 60%, 100%)
           3s:  rect(30%, 0%, 60%, 100%) ease-out
           6s:  rect(12%, 0%, 60%, 100%) ease-in-out
         }
       )

  canvas {
    place vertical {
      fill = canvas
    }

    captions from narration.short.words {
      region = rect(8%, 66%, 84%, 24%)
      style = "shorts"
    }
  }

  mix {
    place gameplay.audio {
      gain = -12dB
    }

    place narration.short.audio
  }
}
```

これはかなり面白い。

`crop()`が静的矩形ではなく、

```text
Property<Rect>
```

を受け取れれば、**手動Reframeは普通のCurve**として表現できる。

つまり、

```text
Crop
Position
Scale
Gain
Opacity
```

を全部、

```text
Property<T>
```

として統一した設計が効く。

---

# 6. ミーム編集

ゲーム実況では実際めちゃくちゃありそう。

例えば死亡Frameで突然停止。

```vel
scene death_joke {
  source death =
    game[9m12s..9m18s]
    |> freeze(
         at: 4.32s,
         for: 2.5s,
         audio: silence
       )

  asset vineboom = audio("./vine-boom.wav")

  canvas {
    place death.video {
      fill = canvas
    }

    text "完" {
      at = 4.32s
      for = 2.5s

      position = (50%, 50%)

      transform.scale = curve {
        0f:  0.2
        8f:  1.3 ease-out
        14f: 1.0 ease-in
      }
    }
  }

  mix {
    place death.audio

    place vineboom at 4.32s {
      gain = -2dB
    }
  }
}
```

これ、かなり素直に書ける。

さらに画面全体を拡大したい。

```vel
canvas {
  place death.video {
    fill = canvas

    transform.scale = curve {
      4.32s:       1.0
      4.32s + 8f:  1.35 ease-out
    }
  }
}
```

つまり**Canvas PlacementもPropertyを持つ**というモデルで十分いける。

---

# 7. パッチノート・記事解説

これも良いストレステスト。

画面キャプチャや画像を使って、

> この行だけ拡大  
> ハイライト  
> 次の項目へスクロール

をする。

```vel
scene patch_notes over narration.patch {
  source page =
    image("./patch-notes.png")

  source paragraph =
    page
    |> crop(rect(12%, 34%, 76%, 22%))

  canvas {
    place page {
      fill = canvas
      fit = contain
    }

    place paragraph at 4s for 7s {
      region = rect(8%, 58%, 84%, 34%)
      fit = contain
    }

    highlight {
      region = paragraph
      at = 5s
      for = 4s
    }

    text "ここが変更点" {
      at = 5s
      for = 4s
      position = (50%, 18%)
    }
  }
}
```

でもWebページのスクロールまで行くと、

```text
Source Image
```

ではなく、

```text
Document / Surface
```

みたいな概念が欲しくなる。

とはいえv0では、スクショを何枚か使えばいい。

---

# 8. マルチカメラ実況

例えば、

```text
Gameplay
Facecam
Handcam
```

が全部同期録画されている。

```vel
scene commentary {
  source gameplay = game[10m..11m]
  source facecam  = face[10m..11m]
  source handcam  = hand[10m..11m]

  canvas {
    place gameplay.video {
      fill = canvas
    }

    place facecam.video {
      region = rect(73%, 4%, 24%, 25%)
      mask = round_rect(radius: 18px)
    }
  }

  mix {
    place mic.audio
  }
}
```

途中だけHandcamへ切り替える。

```vel
canvas {
  place handcam.video at 24s for 6s {
    fill = canvas
  }
}
```

これは問題ない。

ただ、

> 「2秒だけFacecamを全画面にして戻す」

となると、

```vel
place facecam.video at 18s for 2s {
  fill = canvas
}
```

で済む。

**後に書いたPlacementほど前面**というルールがあれば、意外とシンプルね。

---

# 推測（示唆）

この8パターンを殴ると、現VELに必要そうなものがかなり見えてきた。

特に強く感じるのは5つ。

### `Scene`はかなり正しい

どのテーマでも、

```vel
scene foo {
```

で局所的に閉じるのが気持ちいい。

トップレベルSequenceは、

```vel
use intro
use demo
use joke
use outro
```

だけでいい。

これは残したい。

### `Canvas`もかなり正しい

合成系はほぼ全部、

```text
place
layout
text
shape
```

に落ちる。

そしてPlacementの、

```text
position
region
scale
opacity
mask
```

を`Property<T>`で統一できる。

かなり筋がいい。

### Source PipelineだけではCut編集が少し読みにくい

ここは今回一番引っかかった。

```vel
game
|> remove(...)
|> speed(...)
|> freeze(...)
```

で全部書けはする。

でも長尺動画では、

```text
Cut List
```

として見た方が読みやすい。

だから、

```vel
source gameplay = game[...]

edit gameplay {
  remove 12s..18s
  speed 30s..42s = 2.0x
  freeze at 55s for 2s
}
```

みたいな**編集専用のSugar**はかなり欲しい。

内部では全部TimeMapへコンパイルすればいい。

### `Collection`が必要

Montageをやった瞬間、

```text
Source
```

一個ではなく、

```text
List<Source>
List<Placement>
EventSet
SpanSet
```

が一次値として欲しくなる。

これはVELを結構ちゃんとした言語へ押し上げる。

### 時間参照は予想以上に重要

今回だけでも、

```text
Scene local time
Source local time
Sequence time
Event time
Clip edge
Speech word timing
Beat timing
```

が出てきた。

だからVELでは「時間」がただの`Duration`ではなく、かなり強い型になるでしょう。

---

今の感触だと、**次に一番面白いストレステストは「音ハメMontage」**だと思う。

あれだけ、

```text
単一Source
↓
単一Placement
```

という現在の前提を明確に壊してくる。

VELが単なるScene記述DSLから一段上へ行くなら、`Collection / EventSet / Arrangement`をどう扱うかが次の壁になりそうよ。

---

# you asked

message time: 2026-08-19 00:59:45

やってみて

---

# chatgpt response

いいわ。音ハメMontageを**ちゃんと20秒くらい作るつもり**で書いてみる。ここはVELの「Collection / EventSet / Arrangement」が本当に必要なのかを見るにはちょうどいい。

## 事実

想定する編集はこう。

- BGMの16秒区間を使う
- 強拍ごとにカット
- 特に強い4拍では派手なキルシーンを使う
- 途中だけ2倍速→着弾瞬間で等速
- 1か所だけ3分割
- 最後のドロップで白フラッシュ
- ゲーム音は着弾前後だけ残す
- 映像素材は全部別の場所から取る

まず、今までのVELで無理やり書くとこうなる。

```vel
scene montage {
  duration = 16s

  source music = bgm.audio[42s..58s]

  source shot1 = game[2m14s..2m17s]
  source shot2 = game[5m31s..5m34s]
  source shot3 = game[9m02s..9m06s]
  source shot4 = game[12m44s..12m47s]
  source shot5 = game[18m03s..18m07s]
  source shot6 = game[23m18s..23m22s]

  canvas {
    place shot1.video at 0s    for 2s { fill = canvas }
    place shot2.video at 2s    for 2s { fill = canvas }
    place shot3.video at 4s    for 2s { fill = canvas }
    place shot4.video at 6s    for 2s { fill = canvas }
    place shot5.video at 8s    for 2s { fill = canvas }
    place shot6.video at 10s   for 2s { fill = canvas }

    place shot2.video at 12s for 1s { fill = canvas }
    place shot4.video at 13s for 1s { fill = canvas }
    place shot1.video at 14s for 1s { fill = canvas }
    place shot6.video at 15s for 1s { fill = canvas }
  }

  mix {
    place music
  }
}
```

まあ書ける。

でもこれは**「BGMに合わせて編集した」情報を完全に失っている**。

`2s, 4s, 6s...`というマジックナンバーが並んでいるだけ。

ここで曲を差し替えた瞬間に全部死ぬ。

---

# EventSetを入れる

まず音楽から時間上のイベントを得る。

```vel
events beat =
  music
  |> beats()
```

`beat`は単一時間ではなく、

```text
EventSet<Beat>
```

。

中身をイメージすると、

```text
beat[0] = 0.12s
beat[1] = 0.61s
beat[2] = 1.10s
beat[3] = 1.59s
...
```

さらにBeatには、

```text
time
strength
bar
beat
```

くらいのメタデータを持てる。

だから、

```vel
events strong =
  beat
  |> where(strength > 0.8)
```

ができる。

これ自体はかなり自然。

---

# 素材側もCollectionにする

今まで、

```vel
source shot1 = ...
source shot2 = ...
```

だったのを、

```vel
sources shots = [
  game[2m14s..2m17s],
  game[5m31s..5m34s],
  game[9m02s..9m06s],
  game[12m44s..12m47s],
  game[18m03s..18m07s],
  game[23m18s..23m22s],
]
```

にする。

型としては、

```text
Collection<Source>
```

。

ここまでは別に怖くない。

---

# さて、どう配置するか

一番素朴なのは、

```vel
placements cuts =
  shots
  |> arrange(
       at: beat,
       mode: sequence
     )
```

だと思う。

意味は、

```text
shot[0] → beat[0]
shot[1] → beat[1]
shot[2] → beat[2]
...
```

。

ただ問題がある。

**各Clipの長さは何秒？**

次のBeatまでなら、

```vel
placements cuts =
  shots
  |> arrange(
       spans: between(beat),
       mode: sequence
     )
```

の方が正確。

ここで、

```text
beat:
  •     •       •    •

between(beat):
  [-----)[-------)[----)
```

という変換が必要になる。

これ、結構いい概念だと思う。

`EventSet → SpanSet`。

---

# するとMontage本体がこうなる

```vel
scene montage {
  source music = bgm.audio[42s..58s]

  events beat =
    music
    |> beats()

  spans cuts =
    beat
    |> between()

  sources shots = [
    game[2m14s..2m17s],
    game[5m31s..5m34s],
    game[9m02s..9m06s],
    game[12m44s..12m47s],
    game[18m03s..18m07s],
    game[23m18s..23m22s],
  ]

  placements montage =
    shots
    |> arrange(
         into: cuts,
         mode: cycle,
         fit: cover
       )

  canvas {
    place montage {
      fill = canvas
    }
  }

  mix {
    place music
  }
}
```

これ、かなり良い。

**16個のCutを手書きしていない。**

しかも、

```vel
events beat = music |> beats()
```

を別の曲に変えれば再配置できる。

---

## 推測（示唆）

ここで`arrange()`がかなり重要な演算になってきた。

入力：

```text
Collection<Source>
+
SpanSet
```

出力：

```text
Collection<Placement>
```

つまり、

```text
素材集合
+
時間構造
↓
編集された配置集合
```

よ。

これはVELにかなり似合う。

---

# でも実際の音ハメはそんな均一じゃない

例えば、

```text
beat 1    普通
beat 2    普通
beat 3    普通
beat 4    強い → ここだけ派手な素材
```

にしたい。

すると素材にもタグが欲しくなる。

```vel
sources shots = [
  game[2m14s..2m17s] {
    tags = [combat]
  },

  game[5m31s..5m34s] {
    tags = [combat, impact]
  },

  game[9m02s..9m06s] {
    tags = [movement]
  },

  game[12m44s..12m47s] {
    tags = [impact]
  }
]
```

強拍側は、

```vel
events accents =
  beat
  |> where(strength > 0.85)
```

。

そうすると、

```vel
placements base =
  shots
  |> where(not tags.contains(impact))
  |> arrange(into: between(beat))

placements impacts =
  shots
  |> where(tags.contains(impact))
  |> arrange(at: accents)
```

となる。

でもこれは**Placement同士の競合**が出る。

つまり、

```text
base montage
+
impact override
```

。

ここでCanvasの「後勝ち」を使ってもいいが、編集モデルとしてちょっと雑。

むしろ、

```vel
placements montage =
  shots
  |> arrange(
       into: between(beat),
       choose {
         when beat.strength > 0.85:
           prefer tags.contains(impact)

         otherwise:
           prefer tags.contains(combat)
       }
     )
```

くらいにしたくなる。

ただし、ここから急にDSLが高度になる。

---

# もっと現実的に、人間が4箇所だけ指定する

全部を賢くしない方がいいかもしれない。

```vel
placements montage =
  shots
  |> arrange(
       into: between(beat),
       mode: cycle
     )

override montage[3] {
  source = shots.impact1
}

override montage[7] {
  source = shots.impact2
}

override montage[11] {
  source = shots.impact3
}

override montage[15] {
  source = shots.finisher
}
```

これはかなり好き。

つまり、

```text
Ruleで80%作る
↓
Manual Overrideで20%仕上げる
```

という前に出てきた思想がここでも効く。

---

# 「着弾だけゲーム音を出す」

これも音ハメでよくやる。

例えば各素材に手動Markerを置く。

```vel
source shot4 = game[12m44s..12m47s] {
  marker impact = 1.18s
}
```

そしてPlacementがどこへ行っても、

```text
shot4.impact
```

はSource-local。

Placementに変換するとTimeline上の時間が得られる。

なら、

```vel
mix {
  place music

  place montage.audio {
    gain = -inf dB

    gain = envelope {
      around source.marker("impact") {
        before = 80ms
        after = 220ms
        value = -4dB
      }
    }
  }
}
```

とか書きたくなる。

ただ、これは文法がちょっと魔術的ね。

より単純なら、

```vel
mix {
  place music

  for each clip in montage {
    place clip.audio
      from clip.impact - 80ms
      to clip.impact + 220ms
    {
      gain = -4dB
    }
  }
}
```

となる。

ここでついに**`for each`欲しい問題**が出る。

---

# `for each`を解禁するか

私はここで少し態度を変えそう。

以前は、

```text
for
while
```

を入れたくなかった。

でも、

```vel
for clip in montage {
  ...
}
```

はかなり自然。

問題は一般言語化することじゃなく、

```text
有限Collectionに対するmap
```

なら決定論的で安全なのよ。

なので、

```vel
each clip in montage {
  ...
}
```

くらいの**限定反復**はありだと思う。

例えば、

```vel
each clip in montage {
  clip.transform.scale = curve {
    0f: 1.04
    6f: 1.0 ease-out
  }
}
```

これで毎カットの頭に軽いZoom Punch。

便利。

---

# 3分割を一拍だけ入れる

例えば12拍目だけ、

```text
┌─────┬─────┬─────┐
│ A   │ B   │ C   │
└─────┴─────┴─────┘
```

。

これを、

```vel
override montage[11] {
  canvas {
    layout horizontal {
      place shots[1].video
      place shots[3].video
      place shots[5].video
    }
  }
}
```

と書くのはちょっと変。

`montage[11]`はPlacementなのに、急にCanvasになる。

なら**PlacementのcontentはSourceだけでなくCompositionも取れる**べき。

例えば、

```vel
composition triple {
  layout horizontal {
    place shots[1].video
    place shots[3].video
    place shots[5].video
  }
}
```

として、

```vel
override montage[11] {
  source = triple
}
```

。

これはかなり大きい。

つまり`Source`という名前をもっと一般化して、

```text
Visual
├ Media
├ Image
├ Text
├ Generator
└ Composition
```

くらいにした方がいいかもしれない。

---

# 最後にFlash

ドロップの瞬間だけ、

```vel
canvas {
  place montage {
    fill = canvas
  }

  shape rect {
    fill = white

    opacity = curve {
      beat[15]:       0
      beat[15] + 1f:  1
      beat[15] + 5f:  0 ease-out
    }
  }
}
```

これは普通に書ける。

EventをProperty Curveのkeyとして使える。

かなり強い。

---

# かなり完成形に近いMontage

今の議論を全部まとめると、こんな感じ。

```vel
scene combat_montage {
  source music =
    bgm.audio[42s..58s]

  events beat =
    music
    |> beats()

  spans slots =
    beat
    |> between()

  sources shots = [
    impact1,
    movement1,
    kill1,
    impact2,
    movement2,
    kill2,
  ]

  placements montage =
    shots
    |> arrange(
         into: slots,
         mode: cycle,
         fit: cover
       )


  //
  // Human overrides
  //

  override montage[3] {
    source = impact1
  }

  override montage[7] {
    source = impact2
  }

  override montage[15] {
    source = finisher
  }


  //
  // visual rhythm
  //

  each clip in montage {
    clip.transform.scale = curve {
      0f: 1.035
      6f: 1.0 ease-out
    }
  }


  //
  // one special cut
  //

  composition triple {
    layout horizontal {
      place kill1.video
      place kill2.video
      place finisher.video
    }
  }

  override montage[11] {
    source = triple
  }


  canvas {
    place montage {
      fill = canvas
    }

    shape white_flash {
      fill = white

      opacity = curve {
        beat[15]:      0
        beat[15] + 1f: 1
        beat[15] + 5f: 0 ease-out
      }
    }
  }


  mix {
    place music {
      gain = -3dB
    }

    each clip in montage {
      place clip.audio
        from clip.marker("impact") - 80ms
        to clip.marker("impact") + 220ms
      {
        gain = -5dB
      }
    }
  }
}
```

これ、**だいぶ動画編集言語っぽい**。

---

## 推測（示唆）

音ハメをやってみて、かなり重要なものが4つ確定した感じがする。

### 1. `EventSet / SpanSet`は必要

これはAIとは関係ない。

```text
Beat
Marker
Edit Point
Speech Word
Scene Boundary
Manual Cue
```

を同じ時間構造として扱える。

特に、

```vel
events |> between()
```

という変換はかなり基本操作になりそう。

---

### 2. `Collection<T>`も必要

動画編集では、

```text
一個のClip
```

だけでなく、

```text
この10カット
この字幕全部
このBeat全部
このPlacement全部
```

を一括して扱いたい。

なので、

```text
Collection<Source>
Collection<Placement>
EventSet
SpanSet
```

はCore言語にかなり近い。

---

### 3. `arrange()`はVELらしい強いPrimitiveになる

これはかなり気に入った。

```text
何を
+
どの時間スロットへ
+
どんな規則で
↓
Placement集合
```

。

例えば音ハメ以外にも、

```vel
screenshots
|> arrange(into: every(3s))
```

とか、

```vel
broll
|> arrange(into: narration.sentences)
```

とか、

```vel
clips
|> arrange(into: grid)
```

にも広げられる。

時間Arrangementと空間Layoutは対称的になる。

```text
arrange = temporal layout
layout  = spatial arrangement
```

これ、かなり綺麗よ。

---

### 4. 限定的な`each`は入れた方がいい

一般的な制御構文としての`for`はいらない。

でも、

```vel
each clip in montage {
```

という有限集合への一括適用は、動画編集そのもの。

これを禁止すると、

```text
Selector
|> apply
```

という関数型Sugarを延々書くことになる。

むしろVELは、

```vel
each ...
```

を正式に持った方が手書きしやすいと思う。

---

そして一番面白いのは、ここまで来るとVELの中心操作がかなり見えてくること。

```text
Source     素材
Event      時間上の点
Span       時間上の区間
Placement  素材×時間
Composition 素材×空間

between()  Event → Span
arrange()  Source×Span → Placement
layout()   Visual×Space → Composition
```

**時間方向の`arrange`と空間方向の`layout`が対になる。**

これはLatticeのデータモデルとしてかなり美しい骨格になりそうだわ。

---

# you asked

message time: 2026-08-19 01:03:47

UIと、モデル側に必要な知識を組み立ててみるか

---

# chatgpt response

そうね。ここまで来たら、**UIを「人間の編集環境」、モデル側を「同じProject Graphを操作する別のクライアント」**として対称に組み立てると綺麗だと思う。

重要なのは、AIに動画編集ソフトの画面をComputer Useさせる設計にはしないことね。**人間はUIから、モデルは構造化APIから、同じLattice Coreへ触る**。

# 事実

Latticeで今まで出てきた情報は、大きく5種類ある。

```text
1. Source
   media / image / speech / generated visual

2. Temporal
   Sequence / Scene / Event / Span / Placement

3. Spatial
   Canvas / Region / Layout / Transform / Mask / Crop

4. Signal
   Audio / Curve / Beat / RMS / automation

5. Editorial
   Rule / Arrangement / Override / Patch
```

したがってUIもモデルAPIも、この5種類を扱えれば基本的な編集は成立する。

逆に、モデルがFFmpeg filtergraphやGPUI widget treeを知る必要はない。

---

# 推測（示唆）

## まずUIの基本形

私はこうしたい。

```text
┌─────────────────────────────────────────────────────────────┐
│ Project │ main.vel │ Scene: demo │ Render             ▶    │
├──────────────┬──────────────────────────────┬───────────────┤
│              │                              │               │
│ PROJECT      │           PREVIEW            │   INSPECTOR   │
│              │                              │               │
│ Assets       │      ┌────────────────┐      │ Selection     │
│ Scenes       │      │                │      │ Properties    │
│ Speech       │      │     VIDEO      │      │ Relations     │
│ Compositions │      │                │      │ Source        │
│              │      └────────────────┘      │               │
├──────────────┼──────────────────────────────┴───────────────┤
│              │                                              │
│ STRUCTURE    │                 TIMELINE                     │
│              │                                              │
│ main         │  Scene │ hook │ intro │ build │ demo │ ...  │
│ ├ hook       │        ├──────────────────────────────────   │
│ ├ intro      │ Video  │ █████████ ████  ███████████        │
│ ├ build      │ Audio  │ ████████████████████████████        │
│ └ demo       │ Text   │       ███       █████               │
│              │                                              │
├──────────────┴──────────────────────────────────────────────┤
│ Problems │ Agent Diff │ History │ Console │ Render Graph    │
└─────────────────────────────────────────────────────────────┘
```

ただし、これを「常に全部表示するNLE」にしない。

中心領域は切り替える。

```text
Code
Canvas
Timeline
Structure
Render Graph
```

同じProject Graphの**Projection**よ。

---

# 1. Structure View

これはLatticeではかなり重要になると思う。

```text
main
├─ hook                      8.0s
├─ intro                    12.4s
├─ build_explanation        18.7s
├─ demonstration            14.3s
│  ├ gameplay
│  ├ damage_zoom
│  ├ callout
│  └ narration.demo
├─ comparison               11.2s
└─ conclusion               13.8s
```

普通のNLEには弱いビューだけれど、VELでは一次情報に近い。

ここからSceneを並べ替える。

```text
demonstration
↓ drag
comparisonの後へ
```

ならVELでは、

```diff
 sequence main flow {
   use build
-  use demonstration
   use comparison
+  use demonstration
 }
```

になる。

長尺編集だと、Timelineよりこちらを触る時間が結構長くなる気がする。

---

# 2. Timeline View

Timelineは捨てない。ただし**詳細な時間関係を見るProjection**とする。

二段階Zoomが欲しい。

### Scene Timeline

```text
hook | intro | build | demo | compare | conclusion
```

### Scene内部

```text
demo

Video     [ gameplay──────────────────── ]
Damage                     [zoom]
Text                       [callout]
Narration [ narration──────────────── ]
Audio      [ game──────────────────── ]
```

これなら200個のClipを一本の巨大Timelineへ全部並べる必要がない。

IDEでいう、

> repository全体  
> → function内部

みたいなナビゲーションになる。

---

# 3. Canvas View

ここはGPUIを使いたくなる理由がかなり出る。

Preview上のオブジェクトを直接選択。

```text
┌────────────────────────┐
│                        │
│           ┌─────────┐  │
│           │ DAMAGE  │  │
│           └─────────┘  │
│               ↑        │
│             arrow      │
│                        │
└────────────────────────┘
```

damageをクリックすると、

```vel
place damage {
    region = rect(...)
}
```

にジャンプ。

そのままResizeするとコードも変わる。

つまりCanvasは「Preview」以上に、

> **Spatial AST Editor**

なのよ。

これはかなりLattice固有の存在になる。

---

# 4. Code View

普通のIDEらしく、

```text
Go to definition
Find references
Rename
Diagnostics
Autocomplete
Format
Quick Fix
```

は全部欲しい。

ただ、動画言語なので面白い機能が加わる。

カーソルが、

```vel
game[12m35s..12m43s]
```

にあればPreviewがその素材区間になる。

```vel
beat[15]
```

なら波形上で15番目のbeatがハイライト。

```vel
damage
```

ならCanvas上のRegionがハイライト。

つまり**コード上のsymbolに視覚的な意味がある**。

---

# 5. Inspector

Inspectorは単なるProperty Panelではなく、

```text
Selected:
  damage_zoom

Identity
  composition/demo/damage_zoom

Time
  Scene: 5.200s .. 6.700s
  Sequence: 51.830s .. 53.330s

Source
  capture.mp4
  source time: 12:40.200

Spatial
  rect(...)
  parent: demo.canvas

Derived From
  source damage
  crop(...)

Used By
  outline
  callout_arrow
```

くらい見せたい。

つまり**Project Graphの局所ビュー**。

これはそのままモデルに渡せる情報でもある。

---

# 6. Agent Panelはチャット欄だけでは足りない

例えばモデルへ、

> このScene、ダメージ数字が見づらい

と頼む。

Agentは、

```text
Inspect
  demo
  damage
  Canvas

Propose
  - damage regionを15%拡大
  - 背景を少しdarken
  - calloutを上へ移動

Patch
```

を返す。

UIでは、

```text
Agent Change

Canvas                     Before → After
damage.region              40%×28% → 46%×32%
background.opacity         --- → 0.25
callout.position.y         18% → 12%

[Preview Diff] [Apply] [Reject]
```

と見る。

**チャットよりDiffが主役**の方がいい。

---

# ではモデルは何を知る必要があるか

ここが本題ね。

私は5層に分けたい。

---

## Layer 1: Lattice Knowledge

モデルが常時知っている静的な知識。

```text
VEL grammar
VEL type system
Project Graph schema
Edit semantics
Canvas semantics
Time coordinate semantics
available operators
available effects
speech API
```

例えば、

> CropとMaskの違い

や、

> Scene local timeとSource local timeの違い

は毎回プロンプトへ説明しなくていい。

これはCoding Agentでいう「言語仕様・SDK docs」に相当する。

---

# Layer 2: Project Symbolic State

現在のプロジェクト。

ただしVELファイル全部を毎回投入する必要はない。

例えば、

```text
Project
  main
    hook
    intro
    build
    demo
    comparison

Scene demo
  duration 14.3s
  sources:
    gameplay
    damage
  canvas:
    gameplay
    damage_zoom
    callout
  mix:
    gameplay
    narration.demo
```

という**symbol index**をモデルが検索できる。

必要になったら、

```text
get_scene("demo")
get_source("damage")
get_references("damage")
```

で詳細を取る。

コードエージェントがripgrepするのと同じね。

---

# Layer 3: Media Knowledge

ここは全部をモデルcontextへ入れたら死ぬ。

1時間の1440p60映像を当然そのまま渡さない。

モデルには段階的に見せる。

```text
Media
├ metadata
├ thumbnails
├ contact sheet
├ waveform
├ audio events
├ transcript if available
├ existing markers
└ frame / short preview on demand
```

例えばAgentが、

> 12:35付近の攻撃前後を確認したい

なら、

```text
inspect_frames(
  media = game,
  range = 12m33s..12m39s,
  fps = 2
)
```

。

もっと必要なら、

```text
inspect_video(
  range = ...,
  resolution = proxy
)
```

。

つまりモデル側にも**Progressive Disclosure**を入れる。

---

# Layer 4: Current UI Context

これがかなり効く。

モデルには毎回、

```text
Current Scene: demonstration
Current Playhead: 5.42s
Selected: damage_zoom
Hovered: damage region
Active View: Canvas
Visible Timeline Range: 3s..9s
```

くらい渡す。

すると、

> 「これもう少し大きくして」

で、

```text
これ = damage_zoom
```

を特定できる。

人間がIDEでペアプログラミングするときの「今見ているところ」を共有するわけ。

これはコンテキスト量が小さいのに価値が大きい。

---

# Layer 5: Execution Feedback

Agentは書いたVELが正しいと思い込んではいけない。

必ず、

```text
parse
typecheck
resolve
layout
render preview
```

の結果を取れるようにする。

例えば、

```text
error:
Scene "demo"
Canvas coverage gap: 11.2s..14.3s

warning:
damage_zoom is outside safe area at 5.8s

warning:
audio peak = +2.7 dBFS
```

。

モデルはそれを見て修正する。

ここはコーディングエージェントの、

```text
edit → test → inspect
```

と全く同じ。

Latticeなら、

```text
edit → compile → preview → inspect
```

になる。

---

# モデル向けツールはこのくらい欲しい

直接VELを全部編集させるだけでは弱い。

例えば、

```text
Project
-------
project.summary
scene.get
entity.get
entity.references
selection.get

Media
-----
media.info
media.frames
media.preview
media.waveform

Time
----
timeline.inspect
timeline.resolve
events.list
span.inspect

Canvas
------
canvas.inspect
canvas.snapshot
canvas.hit_test

Editing
-------
patch.apply
patch.preview
patch.revert

Validation
----------
project.check
render.preview
render.frame

Search
------
project.query
media.query
```

。

面白いのは、

```text
canvas.hit_test(x, y, time)
```

みたいなの。

モデルがPreview画像上で、

> このDamage Numberは何？

と調べたければ、

```text
hit_test
→ damage_zoom
→ source damage
→ source game@12:40.2
```

まで辿れる。

**Pixels ↔ Project Graphを往復できる**。

これはかなり重要だと思う。

---

# モデルに何を「覚えさせない」か

むしろこっちも重要。

モデルには、

- 全動画内容
- 全フレーム
- 全波形
- 全VEL source
- 全render graph
- GPUI内部状態

を常時渡さない。

必要なときだけ取得させる。

モデルの基本contextは、

```text
Language knowledge
+
Project summary
+
Current selection/context
+
Task
```

程度。

そこから探索。

つまりCoding Agentと全く同じ設計思想になる。

```text
repo全部をpromptに入れない
↓
必要なfileを読む
```

と、

```text
動画全部をpromptに入れない
↓
必要なrangeを見る
```

が対応する。

---

# TTSがあるとモデル側の知識がかなり豊かになる

これは面白い副作用。

生成したSpeechなら、

```text
narration.demo
├ exact text
├ sentence boundaries
├ word timings
├ phoneme timings
├ duration
└ audio
```

が最初から分かる。

Agentが、

> 「“この瞬間”と言うタイミングにFreezeを合わせて」

と言われたら、

```text
narration.demo.words["この瞬間"].start
```

を取得して、

```vel
freeze ... at ...
```

へ繋げられる。

VLMも文字起こしも不要。

**TTSが編集用の時間構造まで生成する**わけ。

これはネイティブ対応の価値が結構大きい。

---

# HumanとAgentの対称性

最終的には、こうなるのが理想。

```text
               Project Graph
              /             \
             /               \
          Human               Agent
            ↓                   ↓
      GPUI Projections      Structured API
            ↓                   ↓
     drag / resize          query / patch
     type VEL               inspect / render
            \                   /
             \                 /
                EditPatch
                   ↓
               Project Graph
```

つまりAgentは「VELファイルを書くbot」ですらない。

**Lattice Projectを操作する正式なparticipant**。

VELはそのProjectを人間が読めて、Git管理できて、決定論的に記述できるserializationでもある。

---

## UIに最初から入れたいもの

MVPなら私は順番をこうする。

```text
1. Structure
2. Code
3. Preview / Canvas
4. Scene Timeline
5. Inspector
6. Problems
7. Agent Diff
```

巨大なPremiere風Timelineは後回しでもいい。

特に最初の編集体験を、

```text
Sceneを作る
↓
VELを書く
↓
Previewを見る
↓
Canvasで位置を動かす
↓
VEL diffを見る
```

まで成立させれば、もうLatticeの思想を検証できる。

その上でAgentにはまず、

```text
selection/context取得
Project Graph query
VEL/Patch生成
preview frame取得
diagnostics取得
```

だけ渡す。

**「モデルに動画編集を教える」より、「モデルが必要な事実をLatticeから問い合わせられるようにする」**方が設計としてずっと強いと思うわ。

ここまで来ると、LatticeのUIとAgent architectureは実は同じ問題を別方向から見ている。どちらも**巨大なProject Graphから、その瞬間必要なProjectionだけを取り出して編集する仕組み**なのよ。

---

# you asked

message time: 2026-08-19 01:06:45

コーディングエージェントを外付けして扱えるようにするなら、MCPか？CLIだけでもいいかな。モデルの繋ぎこみを中でやりたくないんだよなー

---

# chatgpt response

その感覚はかなり正しいと思うわ。**Lattice本体にモデル接続を抱え込まず、外部のCoding Agentから操作される「道具」になる**方が、実装も思想も綺麗。MCPとCLIは競合というより、役割を分けるのがよさそう。

そう。私は **CLIを一次インターフェースにして、MCPは後から薄く被せる**のが一番いいと思うわ。

## 事実

MCPは現在、サーバー側から主に **Tools / Resources / Prompts** を公開でき、標準transportとしてstdioとStreamable HTTPを持っている。つまりLatticeの構造化操作を外部Agentへ見せる用途には確かに合っている。([Model Context Protocol](https://modelcontextprotocol.io/specification/2025-11-25?utm_source=chatgpt.com))

ただしMCPそのものもまだ動いている規格で、2026年7月にも次期仕様のRelease Candidateが出ている。Latticeの中心APIをMCPそのものに合わせて設計する理由はないわ。([Model Context Protocol Blog](https://blog.modelcontextprotocol.io/posts/2026-07-28-release-candidate/?utm_source=chatgpt.com))

そしてCoding Agentなら、だいたい既に「リポジトリを読む・ファイルを書く・CLIを実行する」が本職よ。だから、

```bash
lattice check
lattice inspect scene demo
lattice render frame --scene demo --at 5.2s
```

を実行できれば、それだけで相当なところまで行ける。

## 推測（示唆）

私はこうしたい。

```text
                    lattice-core
                         │
                 ┌───────┴───────┐
                 ↓               ↓
             lattice-cli     Lattice Studio
                 │               │
                 │          local IPC (later)
                 │               │
                 └───────┬───────┘
                         ↓
                  lattice-mcp
                    optional
```

**モデル接続はどこにもない。**

Claude CodeでもCursorでもGrok系Agentでも将来の何かでも、

```text
Coding Agent
     ↓
shell / MCP
     ↓
Lattice
```

というだけ。

この境界はかなり気持ちいい。

---

### 最初はCLIだけで十分

例えばAgentへのREADMEには、極端にはこれだけ書けばいい。

```text
Lattice projects are edited as VEL source.

Useful commands:

lattice check
lattice fmt
lattice inspect
lattice query
lattice render
lattice media
lattice patch
```

Agentが、

> demoシーンのダメージ数字をもっと見やすくして

と言われたら、

```bash
$ lattice inspect scene demo --json
```

```json
{
  "duration": "14.3s",
  "canvas": [
    "gameplay",
    "damage_zoom",
    "callout"
  ]
}
```

↓

```bash
$ lattice inspect damage_zoom --json
```

↓

`.vel`を編集。

↓

```bash
$ lattice check
```

↓

```bash
$ lattice render frame \
    --scene demo \
    --at 5.4s \
    --output /tmp/lattice-preview.png
```

↓

画像を見る。

↓

修正。

これ、**完全にCoding Agentの通常ループ**なのよ。

```text
read
→ edit
→ build/test
→ inspect result
→ edit
```

が、

```text
read VEL
→ edit
→ lattice check
→ lattice render
→ inspect frame
→ edit
```

になるだけ。

専用Agent Runtimeを書く必要がない。

---

# CLIは最初からAgent-friendlyにする

人間用出力だけだと後で困るので、ほぼ全コマンドに、

```bash
--json
```

を持たせたい。

例えば、

```bash
lattice inspect scene demo --json
```

```bash
lattice query \
  'scene("demo").placements' \
  --json
```

```bash
lattice events beats bgm \
  --range 42s..58s \
  --json
```

。

診断も、

```bash
lattice check --json
```

で、

```json
{
  "status": "error",
  "diagnostics": [
    {
      "severity": "error",
      "code": "E1024",
      "scene": "demo",
      "message": "Canvas coverage gap",
      "range": {
        "start": "11.2s",
        "end": "14.3s"
      },
      "source": {
        "file": "main.vel",
        "line": 183,
        "column": 5
      }
    }
  ]
}
```

。

これならAgent側にLattice専用integrationがなくても扱える。

---

# CLIのカテゴリもかなり絞れる

私は最初、

```text
lattice
├─ check
├─ fmt
├─ inspect
├─ query
├─ media
├─ render
└─ patch
```

くらいでいいと思う。

例えば、

```bash
lattice inspect project
lattice inspect scene demo
lattice inspect source gameplay
lattice inspect refs damage_zoom
```

。

メディア：

```bash
lattice media info game

lattice media frame game \
  --at 12m40.2s

lattice media contact-sheet game \
  --range 12m30s..12m50s \
  --every 1s

lattice media waveform narration.demo
```

。

レンダー：

```bash
lattice render frame \
  --scene demo \
  --at 5.2s

lattice render scene demo \
  --proxy

lattice render project \
  --profile youtube
```

これだけでもモデルにかなり目と手を与えられる。

---

# `patch`は面白いが、必須ではない

Agentは普通に`.vel`を書き換えられるので、

```bash
lattice patch ...
```

が最初から絶対必要というわけではない。

むしろv0なら、

```text
Agent edits .vel
↓
VEL parser/typechecker
↓
Git diff
```

で十分強い。

ただ、将来的には、

```bash
lattice patch apply --stdin
```

に、

```json
{
  "base_revision": 103,
  "operations": [
    {
      "op": "set_property",
      "target": "demo.damage_zoom.region",
      "value": "rect(52%, 54%, 44%, 34%)"
    }
  ]
}
```

を食わせられるようにすると、

- GUI
- Agent
- Undo/Redo
- collaboration

が同じEditPatchを共有できる。

これは後から欲しい。

---

# MCPが欲しくなる境界もある

CLIだけでは少し面倒なのが、**Lattice Studioのライブ状態**。

例えばユーザーが、

> これもうちょっと右

と言ったとき、

```text
Current scene = demo
Selected = damage_zoom
Playhead = 5.427s
Active view = Canvas
```

は`.vel`ファイルにはない。

そこで初めて、

```text
Lattice Studio
     ↓ local IPC
live session
```

が欲しくなる。

CLIなら、

```bash
lattice studio context --json
```

としてもいい。

返るのは、

```json
{
  "project": "obex-guide",
  "scene": "demo",
  "selection": ["damage_zoom"],
  "playhead": "5.427s",
  "view": "canvas"
}
```

。

そして、

```bash
lattice studio snapshot \
  --output /tmp/current.png
```

。

**これでもCoding Agentからは十分操作できる。**

---

# その上にMCPを被せればいい

その段階になったら、

```text
MCP tool                   内部
────────────────────────────────────────
project_inspect       → ProjectService
project_query         → ProjectService
media_frame           → MediaService
render_frame          → RenderService
studio_context        → Studio IPC
studio_snapshot       → Studio IPC
apply_patch           → EditService
```

にする。

MCPサーバー自身はほとんどロジックを持たない。

公式MCPでもToolsは「モデルから実行できる機能」、Resourcesはコンテキストデータを公開するものとして分離されているので、LatticeならProject inspectionやmedia metadataをResource寄りに、render/apply patchなどをToolとして出す構成は自然よ。([Model Context Protocol](https://modelcontextprotocol.io/docs/learn/architecture?utm_source=chatgpt.com))

そしてlocal integrationならstdio transportも標準で存在するので、

```json
{
  "command": "lattice",
  "args": ["mcp"]
}
```

みたいな一プロセス型にもできる。([Model Context Protocol](https://modelcontextprotocol.io/specification/2025-11-25/basic/transports?utm_source=chatgpt.com))

---

# むしろ避けたい構造

私はこれは嫌。

```text
Lattice Studio
├─ OpenAI API
├─ Anthropic API
├─ xAI API
├─ Gemini API
├─ model routing
├─ context management
├─ tool loop
├─ token accounting
└─ agent implementation
```

Latticeが突然、

**動画編集環境を作っていたはずなのにAIチャットクライアントを保守するプロジェクト**

になる。

それは明らかに本筋じゃない。

しかも外部Coding Agentの方が、

- Repository理解
- Git
- Shell
- MCP
- Planning
- Subagents
- Model routing

を勝手に進化させてくれる。

Lattice側が追従する必要がない。

---

# 私ならこの順番

```text
Phase 1
VEL + lattice CLI

        ↓

Phase 2
Agent-friendly JSON output
media frame/contact-sheet
render preview

        ↓

Phase 3
Studio local IPC
lattice studio context/snapshot

        ↓

Phase 4
lattice mcp
CLI/Core APIの薄いadapter
```

これがいい。

なので質問への短い答えなら、

> **CLIだけで始めていい。むしろCLIを正規のautomation interfaceにする。MCPは外部AgentとのUXを良くするadapterとして後から載せる。**

だと思う。

特にLatticeはVEL自体がテキストで決定論的だから、Coding Agentとの相性が最初から異常にいいのよ。Agentには`.vel`を書かせ、`lattice check/render/inspect`を実行させればいい。

**モデルを繋ぎ込まないことを、Latticeの設計原則にしてしまっていい**くらいだと思うわ。モデル競争は外で勝手にやっていてもらえばいい。Latticeは優秀な「目と手」を提供する側に徹すればいいのよ。

---

# you asked

message time: 2026-08-19 01:11:43

Undo/Redoとか履歴とかはgitベースで勝手にやれ（連携はつけるけど）、くらいでいいかな？というか、プロジェクトの概念をどこまでやるかみたいな

---

# chatgpt response

うん。私はかなり強く、**「永続的な履歴はGitに任せる。Latticeは履歴管理システムを作らない」**でいいと思うわ。

ただし、`Ctrl+Z`までGitに押しつけるのは少し違う。ここを分けると設計がかなり綺麗になる。

# 事実

動画編集でいう「履歴」には、実際には二種類ある。

```text
操作の取り消し
────────────────
ドラッグを戻す
Cropを戻す
Cutを戻す
Property変更を戻す

プロジェクトの履歴
────────────────
昨日の状態に戻る
試行Aと試行Bを比較
誰が何を変えたか
branchを切る
```

前者がUndo/Redo、後者がVersion Controlよ。

Gitは後者には非常に向いているけれど、

```text
マウスでタイトルを20px右へ動かした
↓
Ctrl+Z
```

までGit commitにするのは明らかに重い。

---

# 推測（示唆）

## Undo/Redoは「セッション内だけ」でいい

Lattice Studioには、

```text
EditTransaction
EditTransaction
EditTransaction
...
```

程度の**in-memory stack**だけ持たせる。

例えばCanvasでドラッグ開始。

```text
mouse down
↓
begin transaction
↓
preview上では100回位置更新
↓
mouse up
↓
1 EditPatchとしてcommit
```

そして、

```text
Ctrl+Z
→ inverse patch

Ctrl+Shift+Z
→ redo patch
```

。

アプリを閉じたらUndo stackは消えてもいい。

昨日の状態へ戻りたければ、

> Gitを使え。

で十分だと思う。

これは機能不足というより、**責務分離として健全**よ。

---

# 「History View」もGitを表示すればいい

Lattice独自履歴：

```text
Lattice History Database
├─ revision 182
├─ revision 183
├─ revision 184
...
```

なんて作らない。

StudioのHistoryタブを開いたら、

```text
Git History

● tweak damage callout
● shorten intro
● add comparison scene
● initial narration
```

を見せる。

そして、

```text
Show VEL Diff
Show Timeline Diff
Show Video Diff
Checkout
Compare
```

だけLattice独自の可視化を足す。

つまり、

> **履歴を所有しない。履歴の動画向けUIを提供する。**

これ、かなり好きね。

---

# で、「Project」をどこまで持つか

私はここもかなり薄くしたい。

極端には、

> **Lattice Project = VELを評価するためのdirectory context**

程度でいい。

例えば、

```text
my-video/
├─ lattice.toml
├─ main.vel
├─ scenes/
│  ├─ intro.vel
│  ├─ demo.vel
│  └─ outro.vel
├─ assets/
│  ├─ logo.png
│  └─ bgm.flac
└─ .lattice/
   └─ cache/
```

。

これがそのままProject。

謎の、

```text
MyVideo.latticeproject
```

みたいなopaque bundleは要らない。

---

## むしろ単一ファイルでも動かしたい

```bash
lattice edit demo.vel
```

だけでも開ける。

`lattice.toml`がなければ、

```text
demo.velがあるdirectory
=
implicit project root
```

。

規模が大きくなったら、

```bash
lattice init
```

して明示Projectへ昇格。

これはCargoなんかに近い感覚ね。

---

# `lattice.toml`に何を書くか

最初は驚くほど少なくていい。

```toml
[project]
entry = "main.vel"

[format]
width = 2560
height = 1440
fps = 60

[audio]
sample_rate = 48000

[paths]
assets = ["./assets", "../captures"]
```

下手するとFormatすら`main.vel`側に書けるから、

```toml
[project]
entry = "main.vel"
```

だけでも成立する。

Project manifestは、

> **VELだけでは表しづらい環境情報**

に限定したい。

---

# Projectに入れるもの / 入れないもの

私はこんな線引きにする。

### Projectの正本

```text
*.vel
lattice.toml
lattice.lock（必要になれば）
プロジェクト内Asset
```

### `.lattice/`に入る派生物

```text
thumbnail
waveform
proxy
TTS cache
analysis cache
render cache
index
```

基本`.gitignore`。

### ローカルUI State

```text
playhead
opened panels
window positions
selected entity
timeline zoom
```

これも、

```text
.lattice/ui-state.json
```

あたりに置いてGit管理しない。

### Latticeが持たない

```text
version history database
branch model
persistent undo journal
collaboration history
独自VCS
```

。

実に平和になるわ。

---

# AssetまでProjectが所有する必要もない

これは動画ではかなり重要。

例えば録画が、

```text
D:\Captures\Warframe\2026-08-18.mp4
```

にあるとして、いちいちProjectへコピーしたくない。

だからVELは普通に、

```vel
asset game =
  media("/captures/Warframe/2026-08-18.mp4")
```

でもいい。

あるいはproject-relative。

```vel
asset logo =
  image("./assets/logo.png")
```

。

つまりProjectは、

> **Assetを参照する**

のであって、

> **全Assetを所有する**

わけではない。

必要になったら、

```bash
lattice assets collect
```

でportable project化すればいい。

---

# ただし「Asset identity」は必要そう

ファイルパスだけだと、

```text
capture.mp4を別ファイルに差し替えた
```

のにcacheがそのままとか起きる。

だから内部的には、

```text
locator
+
fingerprint
```

を持つ。

でもこれもユーザーに巨大なAsset Databaseを管理させる必要はない。

Latticeが勝手に、

```text
.lattice/index
```

へ持てばいい。

正本ではないから消して再構築可能。

---

# `lattice.lock`は将来かなり意味がありそう

これはProject概念の中で唯一、後々重要になるかもしれない。

例えば、

```vel
speech narration using elevenlabs {
    ...
}
```

や、

```vel
events beats = music |> beats()
```

がある。

入力から毎回生成すると結果が変わる可能性があるものについて、

```text
narration:
  engine: ...
  voice: ...
  source hash: ...
  generated artifact: ...

beats:
  analyzer: ...
  version: ...
  resolved events: ...
```

をlockする。

つまり、

```text
main.vel
    意図

lattice.lock
    外部依存・非決定的処理の解決結果
```

。

Gitには両方入れる。

これはpackage lockfileとほぼ同じ思想でいい。

---

# Project Graphも「保存形式」ではない方がいい

ここも割と重要。

```text
*.vel
+
manifest
+
lock
      ↓ compile
Project Graph
```

。

Project Graphはメモリ上のderived representation。

つまり、

```text
project.lattice-db
```

みたいなのを正本にしない。

GPUIはProject Graphを見る。

CLIもProject Graphを見る。

Agentの`lattice inspect`もProject Graphを見る。

でもユーザーが管理するのはVEL。

この一方向性は維持した方がいいと思う。

---

# Git連携はむしろ相当面白くできる

Git自体を再実装しない代わりに、

**Gitを動画編集向けに解釈する**。

例えば、

```bash
git diff
```

相当をLattice Studioでは、

```text
main.vel

Scene changes
────────────────────
intro
  duration     18.4s → 15.2s

demo
  damage_zoom
    region      changed

comparison
  added

Overall
────────────────────
duration      7:42 → 7:38
cuts          31 → 34
```

と見せる。

さらに、

```text
[Preview Before]
[Preview After]
```

。

ここまでやれば、「Gitを勝手に使え」がむしろ強みになる。

GitHub PRで動画編集レビューなんてことも自然にできる。

---

# なのでProject概念を最小化すると

私はたぶんこう定義する。

> **Lattice Projectとは、1つ以上のVEL moduleと、それらの外部依存を解決するためのrootである。**

それだけ。

Projectは、

```text
履歴を持たない
Assetを必ず所有しない
GUI状態を意味論に含めない
Render結果を意味論に含めない
Agent状態を持たない
```

。

持つのは、

```text
VEL source
dependency resolution
format/config
optional lockfile
```

だけ。

かなりUnixっぽくていいでしょう。

---

そしてこれ、Coding Agent外付け方針とも綺麗に繋がる。

```text
Git repo
├─ VEL
├─ lattice.toml
└─ lattice.lock

        ↑
   Coding Agent
     git/edit
        ↑
        │
  lattice CLI
        ↑
        │
 Lattice Studio
```

**Project = 普通のrepo。Latticeはそのrepoを理解する専門IDE。**

私はこれくらいまで薄くするのが一番好きね。

Undo/RedoだけセッションUXとして最低限持って、**「歴史」という重たい概念はGitへ丸投げする**。Latticeが動画編集ソフトである以上、VCSまで発明し始めたら明らかに何かがおかしいわ。

---

# you asked

message time: 2026-08-19 01:15:01

動画編集者ってどういう管理の仕方してるんだ？テッキー向けだから別に忠実である必要はないんだけど

---

# chatgpt response

ある程度の「現場の型」はあるわ。ただし、映像編集者の管理ってソフトごとの差が大きくて、**ソース・プロジェクト・キャッシュ・書き出し・バックアップを分ける**のが共通パターン、という理解が一番近い。今の主要NLEの公式運用も少し見て、Latticeに持ち込むべき部分だけ拾うわ。

そうね。調べてみると、**映像編集者の「プロジェクト管理」はソフトウェア開発ほど一枚岩ではない**。むしろ「ファイル置き場」と「NLE内部の整理」と「派生物」をゆるく分けて運用している感じよ。

## 事実

典型的な構造をかなり抽象化すると、こうなる。

```text
JOB / PROJECT
│
├─ Source Media
│   ├─ Camera
│   ├─ Screen Capture
│   ├─ Audio
│   ├─ Images
│   └─ Graphics
│
├─ NLE Project
│   ├─ Bins
│   ├─ Sequences / Timelines
│   ├─ Markers
│   └─ Metadata
│
├─ Derived
│   ├─ Proxies
│   ├─ Waveforms
│   ├─ Cache
│   └─ Preview renders
│
├─ Exports
│   ├─ rough_v01
│   ├─ rough_v02
│   └─ final
│
└─ Archive
```

Premiereでは、素材そのものと`.prproj`は別物で、プロジェクト内部にはBinを作って素材やSequenceを論理整理する。Adobe自身もSequence/TimelineとExportのversion numberを揃えることを推奨しているし、Binをbackup sequenceの保存場所としても使えるとしている。([Adobe ヘルプセンター](https://helpx.adobe.com/ca/premiere/desktop/organize-media/create-projects/start-projects.html?utm_source=chatgpt.com))

長尺・大規模になるとPremiereの`Productions`はさらに一段上に「複数Projectを束ねるProduction」を置く。例えば映画ならreelごと、TVならepisodeごとにProjectを分割しつつ、素材はProject間で複製せず参照できる。面白いことにAdobeはProductionフォルダの中には**mediaを置かない**よう案内しているので、「編集構造」と「重い実体」をかなり明確に分けているわ。([Adobe ヘルプセンター](https://helpx.adobe.com/premiere/desktop/collaborate-with-others/collaborate-using-productions/about-productions.html?utm_source=chatgpt.com))

DaVinci ResolveにもMedia PoolとBinがあり、例えばフォルダごと素材をImportすると、そのフォルダ構造をBinとして維持できる。つまりこちらも、「ディスク上のMedia」と「編集上の論理整理」という二層がある。([Blackmagic Design Documents](https://documents.blackmagicdesign.com/UserManuals/DaVinci-Resolve-20-Beginners-Guide.pdf?_v=1757574010000&utm_source=chatgpt.com))

AvidはさらにBin文化が強く、Project内に`.avb`というBinファイルを持つ。プロの映像編集では「フォルダ」よりBinが一級の整理単位として扱われてきた面があるわ。([Avid](https://kb.avid.com/pkb/articles/en_US/how_to/How-to-locate-the-Project?utm_source=chatgpt.com))

そしてProxy・Waveform・conformed audioなどは基本的に**消せる派生物**。PremiereのMedia Cacheも`.pek`や`.cfa`などの高速化用ファイルを格納していて、削除すれば必要に応じて再生成される。([Adobe ヘルプセンター](https://helpx.adobe.com/jp/premiere/desktop/troubleshooting/media-issues/manage-media-cache.html?utm_source=chatgpt.com))

完成後には「今使っている素材だけ集めて別の場所へコピーする」というアーカイブ工程もある。Premiere Project Managerには、使用MediaをCollectしてコピーしたり、使用部分だけに絞ってTranscodeしたりする機能がある。つまり制作中は素材が散在していてもよく、**最後にportable bundleへ畳む**運用がちゃんと存在する。([Adobe ヘルプセンター](https://helpx.adobe.com/jp/premiere/desktop/organize-media/create-projects/consolidate-and-archive-projects.html?utm_source=chatgpt.com))

履歴についてはGitのような差分VCSというより、Auto Save、Projectコピー、Sequenceのversion違いなどが中心。Premiereも日時付きAuto Saveを別バックアップProjectとして保存するし、AdobeはSequence/Exportにversion番号を付ける運用を案内している。([Adobe ヘルプセンター](https://helpx.adobe.com/jp/premiere/desktop/get-started/preferences-and-settings/auto-save-preferences.html?utm_source=chatgpt.com))

---

## 推測（示唆）

これを見ると、**Latticeで伝統的NLEの「Project」を忠実に模倣する必要はほぼない**と思う。

むしろ映像編集者の管理法から借りるべきなのは、Project Databaseではなくこの境界だけ。

```text
Source      原本。壊さない
    ↓
Reference   Latticeから参照
    ↓
Edit        VEL
    ↓
Derived     proxy/cache/index
    ↓
Deliverable export
```

これは今まで考えていたLattice構造とかなり一致している。

例えば私はこうしたい。

```text
obex-video/                   ← Git repo / Lattice root
├─ lattice.toml
├─ main.vel
├─ scenes/
│  ├─ hook.vel
│  ├─ explanation.vel
│  └─ demo.vel
│
├─ assets/                    ← 小さい管理対象だけ
│  ├─ logo.svg
│  └─ music.flac
│
├─ .lattice/                  ← 全部消していい
│  ├─ cache/
│  ├─ proxy/
│  ├─ waveform/
│  ├─ thumbnails/
│  └─ index/
│
└─ exports/
```

巨大な録画は、

```vel
asset game =
    media("D:/Captures/Warframe/2026-08-18.mp4")
```

で外部参照。

これなら1時間100GBの動画をGit repoに入れるような奇行をしなくて済む。

そしてLattice上の**Binに相当する概念は、私は永続的な特別オブジェクトにしなくてもいい**と思う。

たとえば、

```vel
group combat = [
    boss_fight,
    eximus_test,
    finisher_test,
]

group broll = [
    arsenal,
    navigation,
    simulacrum,
]
```

でもいいし、

```vel
sources
  |> where(tags.contains("combat"))
```

でもいい。

UI上では、

```text
Sources
├─ Combat
├─ UI
├─ B-roll
└─ Audio
```

と見せても、これは**Project GraphへのSaved Query/View**に近くていい。

つまり映像編集者の`Bin`を、

> folder-like mutable container

ではなく、

> **source collection / view**

として再解釈できる。

これはテッキー向けLatticeにはむしろ合う。

---

### Archiveだけは既存NLEからそのまま盗みたい

例えば、

```bash
lattice collect
```

。

すると、

```text
dist/obex-video/
├─ main.vel
├─ lattice.lock
├─ media/
│  ├─ capture-used.mov
│  ├─ build.png
│  └─ bgm.flac
└─ manifest.json
```

を作る。

オプションで、

```bash
lattice collect --used-only
lattice collect --copy
lattice collect --trim-media
lattice collect --proxy
```

。

これはPremiereのProject Managerとほぼ同じ問題を解く。([Adobe ヘルプセンター](https://helpx.adobe.com/jp/premiere/desktop/organize-media/create-projects/consolidate-and-archive-projects.html?utm_source=chatgpt.com))

制作中は、

> **好きな場所から参照しろ**

完成後は、

> **必要なら自己完結型に固めろ**

でいい。

---

そして、この調査を踏まえると私は前よりさらに、

> **Lattice Project = 「プロジェクトファイル」ではなくWorkspace**

という呼び方の方がしっくり来る。

```text
Workspace Root
├ VEL modules
├ config
├ optional lock
└ references to media
```

であって、Projectの実体を一個のDBやバンドルにする必要はない。

既存の映像編集も実は、巨大Media・論理的な編集Project・再生成可能Cache・最終Deliverableという**かなりファイルシステム的な世界**なのよ。そこへLatticeはGitとテキスト言語を持ち込む、と考えると、それほど異端でもない。

むしろ異端なのは、**`sequence_final_v7_REAL_FINAL.prproj`をGit commitに置き換えられること**の方でしょう。そこは積極的に異端でいいわ。

---

# you asked

message time: 2026-08-19 01:17:07

これは横道なんだけど、同じような操作の表現方法がたくさんあったほうがいいと思う？perlみたいに

---

# chatgpt response

うん、**ある程度は多い方がいい**と思う。ただしPerlをそのまま真似るというより、

> **There’s more than one way to *think about* it.  
> でも、内部意味論は一つ。**

くらいがVELにはちょうどいいと思うわ。

## 事実

同じ操作の書き方を増やすと、手書きではかなり楽になる一方で、

- formatterがどの形へ戻すか
- GUI操作をどのコードへ逆変換するか
- Agentがどの書き方を選ぶか
- diffが無意味に揺れないか
- ドキュメントで何を「標準形」と教えるか

という問題が増える。

VELは特に**GUI ↔ Codeの往復**をしたいから、普通のプログラミング言語よりこのコストが大きいわ。

たとえば全部許すと、

```vel
clip |> speed(2x)
```

```vel
clip.speed = 2x
```

```vel
speed clip = 2x
```

```vel
retime clip by 2x
```

が全部同じ意味になり得る。

人間には親切そうだけれど、Lattice StudioがGUIで速度を2倍にしたとき、**どれを生成すればいいんだ**という問題になる。

---

## 推測（示唆）

だから私は、**「同義語を増やす」のは反対、「異なる思考様式から同じ意味へ到達できる」のは賛成**ね。

かなり重要な違いよ。

### 例えばCrop

素材変換として考えるなら、

```vel
source face =
  camera
  |> crop(rect(10%, 10%, 40%, 40%))
```

これは自然。

でもCanvas上で既に置かれたものを触っているなら、

```vel
place face {
  crop = rect(10%, 10%, 40%, 40%)
}
```

も欲しくなるかもしれない。

これらは単なる二種類のスペルではなく、

```text
Source PipelineとしてのCrop
Canvas Placementに適用するCrop
```

という**操作する対象・フェーズが違う**。

内部的には同じCrop nodeに収束してもいい。

こういう多様性はかなり歓迎したい。

---

### 時間編集も同じ

長尺素材を加工するときは、

```vel
edit gameplay {
  remove 12s..18s
  speed 30s..40s = 2x
}
```

が読みやすい。

一方、式の途中なら、

```vel
source gameplay =
  game[0s..2m]
  |> remove(12s..18s)
  |> speed(30s..40s, 2x)
```

でもいい。

これも、

> block型の編集リスト  
> pipeline型の変換

という二つの見方がある。

私は**両方あっていい**と思う。

---

# 逆に、これはいらない

例えば、

```vel
remove clip
delete clip
drop clip
erase clip
```

を全部同義にする。

これはPerl的な自由さの悪い部分になりやすい。

意味の違いがないなら、

```vel
delete
```

一個でいい。

同様に、

```vel
at
from
starting
beginning
```

みたいな英語表現の揺れも増やしたくない。

**語彙は少なく、構造の選択肢は複数**がいいと思う。

---

# VELで許したい「複数の道」

私は大体この4つを想定する。

### 1. 宣言

```vel
clip demo {
  speed = 2x
  opacity = 0.8
}
```

状態を記述する。

### 2. Pipeline

```vel
game
  |> crop(...)
  |> speed(2x)
  |> blur(4px)
```

加工の順序を記述する。

### 3. Edit block

```vel
edit gameplay {
  remove 12s..18s
  freeze at 32s for 2s
}
```

人間の編集行為として記述する。

### 4. Collection operation

```vel
each clip in montage {
  clip.opacity = 0.8
}
```

または、

```vel
montage
  |> where(duration < 500ms)
  |> delete()
```

集合に対して操作する。

これらはかなり違う脳の使い方をする。

なのに最終的には、

```text
Project Graph
+
TimeMap
+
Properties
+
Placements
```

へ落ちる。

これはVELにとって良い冗長性だと思う。

---

# さらにPerlっぽくするなら「文脈依存Sugar」

例えば、

```vel
scene demo {
  game[10s..20s]
}
```

を、

```vel
scene demo {
  source implicit_source = game[10s..20s]

  canvas {
    place implicit_source.video { fill = canvas }
  }

  mix {
    place implicit_source.audio
  }
}
```

のSugarにしてもいい。

つまり簡単なSceneなら、

```vel
scene intro {
  game[1m..1m20s]
}
```

だけで済む。

複雑になったら明示形へ降りる。

これ、私はかなり好き。

Perlの良いところって「同じことのスペル違い」だけじゃなくて、**簡単なことは雑に短く書けて、必要なら露骨に詳細を書くところまで降りられる**ことでもあるから。

---

# Canonical Formを一つにする必要も、実はないかもしれない

ここは少し面白い。

最初は、

> `lattice fmt`ですべて一つの記法へ正規化

と考えていたけれど、**Sugarまで全部潰す必要はない**と思う。

例えば、

```vel
game |> crop(...) |> blur(...)
```

をformatterが勝手に、

```vel
source x {
  crop = ...
  blur = ...
}
```

へ書き換えたら、普通に腹が立つ。

Formatterは、

- whitespace
- indentation
- block layout
- list formatting

だけをcanonicalizeして、

**意味的に等価な別構文へのrewriteはしない**。

それは、

```text
Refactor → Convert to pipeline
Refactor → Expand scene
Refactor → Collapse to shorthand
```

みたいなIDE commandにすればいい。

この方がLatticeっぽい。

---

# Agentとの相性も意外と悪くない

むしろAgentには、

> **周囲の記法に合わせろ**

とすればいい。

例えば既存Sceneが、

```vel
edit gameplay { ... }
```

中心なら、そのスタイルを維持。

Pipeline中心ならPipeline。

そしてLattice自身がASTベースで、

```bash
lattice style main.vel --json
```

みたいに、

```json
{
  "preferred_source_transform": "pipeline",
  "scene_style": "explicit",
  "collection_style": "each"
}
```

を返せても面白い。

ただこれはずっと後でいい。

---

私ならVELの設計原則を一行で、

> **One semantic core, several useful projections.**

にすると思う。

これ、Lattice全体の思想とも一致するのよ。

```text
Project Graph
├ Code
├ Timeline
├ Canvas
└ Structure
```

と同じように、コードの中にも、

```text
同じ編集意味論
├ 宣言として見る
├ Pipelineとして見る
├ Edit操作として見る
└ Collection変換として見る
```

という複数Projectionがあっていい。

だから**Perlっぽい多様性はかなりあり。ただし「好きな英単語で同じことを書ける」方向ではなく、「同じ編集を異なる自然な抽象度から表現できる」方向に振りたい**わ。これは結構VELの個性になると思う。

---

# you asked

message time: 2026-08-19 01:18:42

わかってきた。どちらかといえばシュガーや記法の問題か

---

# chatgpt response

そうね。**「同じ意味を複数の抽象度で自然に書ける」問題であって、意味論そのものを何種類も持たせる話ではない**、という整理が近いわ。

## 事実

例えば、

```vel
source demo =
  game[10s..20s]
  |> speed(2x)
  |> crop(rect(...))
```

と、

```vel
source demo = game[10s..20s] {
  speed = 2x
  crop = rect(...)
}
```

と、

```vel
edit demo {
  speed = 2x
  crop rect(...)
}
```

を許したとしても、コンパイラ内部では全部、

```text
Source
 ↓
TimeTransform(speed=2x)
 ↓
Crop(rect=...)
```

へ落とせる。

つまり、

```text
Surface VEL
    ↓ desugar / lower
Core VEL
    ↓
Project Graph
```

という構造にすればいい。

この場合、複雑になるのは主にParser、formatter、IDE refactoringであって、**編集意味論そのものは増えない**。

---

## 推測（示唆）

たぶんVELは、最初からこの二層を意識した方がいいと思う。

### Core VEL

かなり無骨。

```vel
source demo = transform(
  crop(
    retime(
      slice(game, 10s..20s),
      rate: 2x
    ),
    rect(...)
  )
)
```

人間が日常的に書く必要はないけれど、意味は極めて明確。

### Surface VEL

好きな書き味を提供する。

```vel
source demo =
  game[10s..20s]
  |> speed(2x)
  |> crop(rect(...))
```

あるいはSceneなら、

```vel
scene demo {
  game[10s..20s] {
    speed = 2x
  }
}
```

これらをCoreへdesugarする。

この構造なら、後からシュガーをかなり大胆に増やせる。

---

特にVELでは、**抽象度に応じたSugar**が効きそう。

単純な動画なら、

```vel
scene intro {
  game[10s..20s]
}
```

少し触るなら、

```vel
scene intro {
  game[10s..20s] {
    speed = 1.2x
  }

  text "Hello" at 2s for 3s
}
```

もっと複雑なら、

```vel
scene intro {
  source gameplay =
    game[10s..20s]
    |> speed(1.2x)
    |> crop(...)

  canvas {
    place gameplay.video { ... }
  }

  mix {
    place gameplay.audio { ... }
  }
}
```

さらに異常なことをするなら、

```vel
composition ...
events ...
spans ...
placements ...
```

まで降りる。

この、

> **簡単なものは簡単に、複雑になった部分だけ低レベルへ降りられる**

のが、たぶんPerlっぽさとして取り込みたい本体ね。

逆に避けたいのは、

```vel
delete foo
remove foo
drop foo
erase foo
```

みたいな単なる語彙の重複。

これは表現力ではなく、ただ覚えることを増やしているだけ。

だからVELでは、

```text
意味論の入口は少ない
↓
記述の抽象度は多い
↓
全部同じCoreへ落ちる
```

がよさそう。

そしてこれならLattice Studioにもかなり面白いことができる。

例えばコードを選んで、

```text
Refactor
├ Expand shorthand
├ Collapse shorthand
├ Convert to pipeline
├ Convert to edit block
└ Extract composition
```

みたいに**同じ編集を別の記法へ機械的に変換**できる。

これ、普通の言語でいうformatterより一段面白くて、Canvas/Timeline/Codeが同じProject GraphのProjectionだというLatticeの思想とも完全に揃うわ。

つまり、だいぶ整理するとVELは**「意味論は保守的、表面構文は贅沢」**くらいがちょうどよさそう。

---

# you asked

message time: 2026-08-19 01:24:09

リント・コンパイラについてもっと考えてみるか

---

# chatgpt response

そうね。ここはLatticeのかなり中核になるわ。**コンパイラを「動画を書き出すもの」ではなく、VEL・Project Graph・Previewをつなぐ意味論エンジン**として考えると、リントまで一気に整理できそう。

そうね。ここはLatticeの中でもかなり面白いところよ。私は**「コンパイラ」と「レンダラー」を明確に別物にする**ところから始めたい。

## 事実

普通のプログラミング言語と同じく、VELでも少なくとも次は分離できる。

```text
Surface VEL
    ↓ parse
CST / AST
    ↓ name resolution + desugar
Core VEL
    ↓ type check
Project Graph
    ↓ resolve
Resolved Project
    ↓ render compile
Render Graph
    ↓ backend
frames / audio
```

ここで重要なのは、

> `lattice check` は動画を書き出さなくても相当なことを検査できる

という点ね。

例えば、

```vel
scene demo over narration.demo {
  source game = capture[10s..18s]

  canvas {
    place game.video
  }
}
```

で、`narration.demo`が11.2秒なら、

```text
game = 8.0s
scene = 11.2s
```

なので、実際にH.265を書き出さなくても「3.2秒映像が足りない」は分かる。

---

# 推測（設計案）

私はコンパイルを**三段階**に分けるのがかなり良いと思う。

## 1. Structural Compile

外部処理を実行せずにできるところまで。

```text
parse
↓
desugar
↓
typecheck
↓
Project Graph構築
↓
symbolic validation
```

例えば、

```vel
scene foo {
  source x = game[10s..20s]
}
```

なら、この時点でかなり決まる。

一方、

```vel
speech intro using narrator {
   ...
}
```

の長さがまだ生成されていなければ、

```text
intro.duration = unresolved
```

でもいい。

つまりコンパイラは**未解決値を許す**。

---

## 2. Resolve

TTSやBeat解析など、「実行しないと分からないもの」を具体化する。

```text
Speech
Beat Detection
Scene Detection
External Generator
etc.
        ↓
Resolved Artifacts
```

例えば、

```vel
events beat =
  music |> beats()
```

が、

```text
beat[0] = 0.121s
beat[1] = 0.619s
beat[2] = 1.108s
...
```

になる。

必要なら`lattice.lock`へ固定。

これで、

```text
Project Graph
        ↓
Resolved Project Graph
```

になる。

この境界はかなり大事ね。

**コンパイラが勝手にAIやTTSへアクセスしない。**

---

## 3. Render Compile

ここで初めて、

```text
Scene
Canvas
Mix
TimeMap
Composition
```

を実際の処理グラフへ落とす。

```text
capture.mp4
    ↓ decode
   crop
    ↓
  scale ─────┐
             ├ composite
 title ──────┘
                  ↓
                encode
```

FFmpeg backendならFFmpeg用。

将来的にGPU-native rendererがあればそちら用。

だからCore VELに、

```text
FFmpeg filter_complex
```

が漏れない。

---

# Core VELがかなり重要になる

Surface syntaxは贅沢にしていいという話をしたけれど、その代わりCoreはかなり冷酷にしたい。

例えば、

```vel
scene intro {
  game[10s..20s] {
    speed = 2x
  }
}
```

も、

```vel
scene intro {
  source x =
    game[10s..20s]
    |> speed(2x)

  canvas {
    place x.video { fill = canvas }
  }

  mix {
    place x.audio
  }
}
```

も、desugar後は同じCore。

そうするとlintもSurface記法ごとに実装しなくて済む。

```text
Surface A ─┐
Surface B ─┼→ Core → semantic lint
Surface C ─┘
```

これはシュガーを増やすならほぼ必須でしょう。

---

# 型システムはかなり強くしたい

VELでは一般言語以上に**次元付きの型**が効く。

例えば、

```text
Duration
SceneTime
SourceTime
SequenceTime

Pixels
NormalizedCoordinate
Degrees

GainDb
Ratio
Frequency

Video
Audio
Image
Visual
Composition

EventSet<T>
SpanSet<T>
Collection<T>
Signal<T>
Property<T>
```

だから、

```vel
gain = 20px
```

は型エラー。

当然。

さらに、

```vel
source_time + scene_time
```

もそのままではエラーにしていいと思う。

見た目は両方`5s`でも、座標系が違うから。

```text
SourceTime(5s)
≠
SceneTime(5s)
```

ここを雑にすると、動画編集特有の「なぜ3フレームずれた？」が大量発生する。

---

# 座標系も型にしたい

例えば、

```text
SourceSpace
CanvasSpace
PlacementSpace
NormalizedSpace
PixelSpace
```

。

```vel
crop(rect(...))
```

がSourceSpaceを期待しているのに、

Canvas上のRegionをそのまま渡したら、

```text
error:
expected Region<SourceSpace>
found    Region<CanvasSpace>
```

と出る。

これ、ものすごく助かると思う。

After Effects的な世界でありがちな、

> 「この座標、どの座標系だ？」

をコンパイラに殺させる。

---

# LintはCompiler Errorとは分ける

ここはかなり明確にしたい。

## Error

意味が定まらない・レンダリング不能。

```text
E001 undefined source "gameplay"

E014 source range 15s..30s exceeds media duration 22s

E031 circular composition reference

E044 expected Audio, found Video

E052 scene time mixed with source time

E081 unresolved external value required for render
```

これは止める。

---

## Warning

レンダリングはできるが、かなり怪しい。

例えば、

```text
W101 canvas is uncovered from 8.2s..9.4s

W112 source is scaled 4.7× above native resolution

W121 audio may clip: predicted peak +2.1 dBFS

W130 caption extends outside title-safe region

W142 media fingerprint changed since last resolve

W150 TTS result differs from lattice.lock
```

---

## Advice

完全に品質・スタイル寄り。

```text
A201 scene contains 14 manual absolute time literals

A205 repeated transform can be extracted into a rule

A220 identical crop occurs in 6 scenes

A231 source "game" is declared but unused

A240 montage override index 14 depends on collection ordering
```

この辺はかなり「IDE」っぽくなる。

---

# Lattice固有のlintが面白い

例えば、

```vel
override montage[15] {
  source = finisher
}
```

。

これは合法。

でも`montage`へ新しい素材を途中挿入したら、15番目が変わるかもしれない。

そこで、

```text
warning LAT210:
positional override depends on unstable collection index

  override montage[15]
                   ^^

consider assigning an id to this slot
```

。

これは普通のプログラミング言語にはないタイプのlintね。

---

# 時間についても相当Lintできる

例えば、

```vel
text "ここ！" at 5s for 1s
```

がSceneの最後を100ms超える。

レンダラーが勝手に切るより、

```text
warning LAT-TIME-103

"text" extends 100ms past scene boundary

scene: demo
scene duration: 6.000s
placement: 5.000s..6.100s
```

。

あるいは、

```text
fix:
- shorten to scene end
- extend scene by 100ms
```

をQuick Fixとして出せる。

---

# そしてDiagnostics自体を多次元にしたい

これがLatticeっぽいところ。

普通のCompiler Diagnosticには、

```text
file
line
column
```

しかない。

Latticeでは、

```text
Source location
+
Project entity
+
Temporal range
+
Spatial region
```

を持たせられる。

例えば、

```json
{
  "code": "LAT-SPATIAL-117",
  "severity": "warning",

  "source": {
    "file": "demo.vel",
    "line": 42,
    "column": 5
  },

  "entity": "demo.damage_zoom",

  "time": {
    "scene": "demo",
    "start": "5.2s",
    "end": "6.7s"
  },

  "region": {
    "space": "canvas",
    "rect": ["82%", "73%", "31%", "22%"]
  },

  "message": "placement extends outside canvas"
}
```

するとIDEで診断をクリックしたとき、

```text
Code
→ 該当行

Timeline
→ 5.2sへ移動

Canvas
→ 該当領域を赤枠

Inspector
→ damage_zoomを選択
```

まで全部一発。

**Compiler DiagnosticがUIナビゲーションの共通プロトコル**になる。

これはかなり強い。

---

# Linterに「深さ」を持たせてもいい

全部のlintを毎キー入力で走らせたら重い。

だから三段階。

### Fast

毎編集。

```text
syntax
types
references
ranges
graph invariants
```

数ms〜程度を狙う。

### Media

ファイルmetadataまで見る。

```text
resolution
duration
codec
sample rate
missing media
```

。

### Deep

実際にdecode / analyseする。

```text
audio clipping
black frames
silence
pixel bounds
caption contrast
render errors
```

。

CLIなら、

```bash
lattice check
lattice lint
lattice lint --deep
```

くらいでもいい。

---

# Preview Compileは部分的にできるべき

GPUI IDEとの相性を考えると重要。

例えば、

```vel
scene demo {
```

のCrop値を変えた。

この変更で、

```text
hook
intro
build
comparison
outro
```

まで再compileする理由はない。

なので依存グラフを持って、

```text
demo.vel changed
      ↓
scene/demo dirty
      ↓
composition/demo dirty
      ↓
preview/demo dirty
```

だけ再構築。

さらに、

```text
Cropだけ変更
```

なら、素材decode cacheすら再利用できる。

ここは最終的には**ビルドシステムに近くなる**。

---

# キャッシュキーもCompilerが握る

例えば、

```text
Source fingerprint
+
operation
+
parameters
+
backend version
```

からhash。

```text
crop(game@abc123, rect(...))
       ↓
cache key f0a8...
```

同じ演算なら再利用。

TTSも、

```text
engine
voice
text
parameters
version
```

でkeyを作る。

だから、

```text
.lattice/cache/
```

は全部消していいけれど、普段は高速。

Bazel/Nixっぽい発想を部分的に持ち込めるわ。

---

# `lattice check`はAgent APIとしても重要

Coding Agentには例えば、

```bash
lattice check --json
```

だけで、

```json
{
  "errors": 0,
  "warnings": 2,
  "affected_scenes": ["demo"],
  "diagnostics": [...]
}
```

。

その後、

```bash
lattice render frame \
  --scene demo \
  --at 5.4s
```

。

つまり外部Agentから見ても、

```text
edit
→ compile
→ diagnostics
→ preview
```

という完全に馴染みのあるループになる。

これはモデルを内蔵しない方針とも非常に相性がいい。

---

# スタイルLintもあっていい

これは強制ではなく、かなりPerl的なSurface VELとのバランスになる。

例えばプロジェクト設定で、

```toml
[lint.style]
prefer_pipeline = true
max_absolute_times_per_scene = 5
prefer_named_regions = true
```

。

すると、

```vel
game |> crop(...) |> speed(...)
```

を推奨したり、

```text
advice:
consider naming this repeated region
```

と出せる。

言語としては自由。

**プロジェクトとしては好みを統一できる。**

これは表面構文を豊かにしても混乱を抑えられる一つの方法ね。

---

# 私ならコンパイラの責務をここまで持たせる

```text
Parser
  ↓
Surface AST
  ↓
Desugar
  ↓
Core VEL
  ↓
Name / Module Resolution
  ↓
Type & Unit Checking
  ↓
Project Graph
  ↓
Semantic Validation
  ↓
External Resolution
  ↓
Resolved Project
  ↓
Render Planning
  ↓
Backend Graph
```

Lintは横から、

```text
       ┌ Syntax lint
       ├ Type lint
Graph ─┼ Temporal lint
       ├ Spatial lint
       ├ Media lint
       ├ Audio lint
       ├ Determinism lint
       ├ Performance lint
       └ Style lint
```

と刺さる。

つまりコンパイラは、VELをFFmpegに翻訳するだけのものじゃない。

> **「この動画プロジェクトは何を意味していて、矛盾していないか」を理解する唯一の権威**

にする。

レンダラーもGPUIもCLIもAgentも、その判定に従う。

私はここをLatticeの中心に置きたいわ。場合によっては**レンダリング品質より先にコンパイラ品質を作り込む価値がある**くらい。VELがコンパイル時に「時間・空間・音・外部生成物の矛盾」をちゃんと指摘してくれるなら、それだけでも既存NLEとはかなり違う編集体験になる。

---

# you asked

message time: 2026-08-19 01:26:10

今更だけど、Moonbitみたいな他の言語に乗っかったほうがいい可能性ってあると思う？

---

# chatgpt response

あると思うわ。むしろ今までVELを「独自言語」として詰めてきたからこそ、**どこまでを既存言語に寄生させられるか**を一度見た方がいい。MoonBitなら特に「静的型・コンパイル・Wasm/JS/Native backend」があるので、乗る価値は普通のスクリプト言語より大きい。

今のMoonBitを見ると、かなり誘惑はあるわ。パイプ、ラベル付き/省略引数、trailing-lambda的な書き方、newtype相当があるので、**VELの表面構文のかなりの部分を「ライブラリDSL」として作れる**。ただし、独自キーワードや独自構文を足すための一般的なmacro機構は現時点の言語仕様では使える機能として出ておらず、`macro`は予約語扱いなのが大きな境界になる。

ある。しかも今までVELを詰めた結果、**「どこを既存言語に乗せられて、どこは乗せると失うか」**がかなり見える段階になったと思うわ。

## 事実

MoonBitは現在もbeta-previewだが、Wasm / Wasm GC / JS / native系の複数backendを持つ汎用言語で、FFIも提供している。Wasm Component Model向けのWIT/bindgen経路も公式に用意されている。 ([MoonBit Documentation](https://docs.moonbitlang.com/en/latest/language/index.html?utm_source=chatgpt.com))

VEL的に嬉しい言語機能もかなりある。

- `|>` / `<|` のpipe
- trailing-lambda的な記述
- labeled / optional arguments
- custom constructor
- 型・enum・pattern matching
- call siteの`SourceLoc`を自動取得する仕組み

が既にある。特に`SourceLoc`は、DSLで生成したProject Graphのnodeに「このMoonBitコードから生成された」を保持するのに使えそう。 ([MoonBit Documentation](https://docs.moonbitlang.com/en/latest/language/fundamentals.html?utm_source=chatgpt.com))

一方、現在のMoonBit仕様では`macro`は予約語で、Rustのproc macroのように自由な独自構文を実装する仕組みとして公開されているわけではない。したがって、

```vel
scene demo over narration.demo {
    ...
}
```

そのものをMoonBit上に生やすことは難しく、基本は**ライブラリDSL**になる。 ([MoonBit Documentation](https://docs.moonbitlang.com/en/latest/language/error_codes/E0031.html?utm_source=chatgpt.com))

例えば雰囲気は、

```moonbit
fn main {
  sequence("main") <| fn(seq) {
    seq.scene("demo", over=narration.demo) <| fn(scene) {
      let gameplay =
        game
        |> slice(start=t("12:35"), end=t("12:43"))

      scene.canvas() <| fn(canvas) {
        canvas.place(gameplay.video, fill=true)
      }

      scene.mix() <| fn(mix) {
        mix.place(gameplay.audio, gain=db(-9))
        mix.place(narration.demo.audio)
      }
    }
  }
}
```

くらいになるでしょう。

悪くない。でも、

```vel
scene demo over narration.demo {
  source gameplay = game[12m35s..12m43s]

  canvas {
    place gameplay.video {
      fill = canvas
    }
  }
}
```

とは明らかに違う。

---

# 推測（示唆）

私は今の時点なら、**VELそのものをMoonBitに置き換えるのは51:49で反対**。

でも、

> **独自言語を作らず既存言語に乗る案を真剣に検証する価値はかなりある**

と思う。

理由は、我々がこれから自作しようとしているものを並べると恐ろしいから。

```text
Lexer
Parser
CST
AST
Formatter
Type checker
Module system
Name resolution
LSP
Completion
Rename
Go-to-definition
Semantic highlighting
Diagnostics
Refactoring
Package/module handling
```

動画編集ソフトを作ろうとしていたら、いつの間にか**言語処理系メーカーになっていた**というありがちな遭難ルートよ。

MoonBitに乗ればこのかなりの部分を無料で得られる。

---

## しかも型システムをそのまま利用できる

例えばVEL用SDK側に、

```moonbit
struct Duration(...)
struct SceneTime(...)
struct SourceTime(...)
struct GainDb(...)
struct Pixel(...)
struct Normalized(...)
struct Region[Space](...)
```

を作る。

すると、

```moonbit
gain = px(12)
```

や、

```moonbit
source_time + scene_time
```

のようなミスをMoonBitコンパイラ自身にかなり殺させられる。

これは相当大きい。

独自VELなら、この型checkerをこちらで書くことになるからね。

---

# ただし、一番惜しいのが「時間リテラル」

VELでは、

```vel
12m35s
8f
500ms
-12dB
50%
```

を直接書ける。

MoonBitをそのまま使えば、おそらく、

```moonbit
minutes(12) + seconds(35)
frames(8)
ms(500)
db(-12)
percent(50)
```

みたいになる。

ある程度method/operator sugarで改善できても、

```vel
game[12m35s..12m43s]
```

の気持ちよさには勝ちにくい。

そしてこれは見た目の問題だけじゃない。

**動画を手書きする言語では、単位付きliteralがものすごく重要**なのよ。

CSSで、

```css
width: 24px;
opacity: 80%;
```

が自然なのと同じ。

---

# もう一つ大きいのが、Lintの支配権

例えばLattice独自の、

```text
warning:
Canvas coverage gap 8.2s..9.4s

warning:
this Region<CanvasSpace>
cannot be used as Region<SourceSpace>

advice:
14 absolute time references in one scene

advice:
this positional montage override is unstable
```

みたいな診断。

MoonBitの型に載せられるものはMoonBitの診断になる。

でも動画編集特有のsemantic lintは結局、

```text
MoonBit
 ↓ execute
Project Graph
 ↓
Lattice semantic checker
```

になる。

つまり、

```text
MoonBit compiler
+
Lattice compiler
```

の二段構成になる。

これは別に悪ではない。

むしろ`SourceLoc`を使ってProject Graph nodeにcall-site情報を保存すれば、

```text
Project Graph diagnostic
↓
元の .mbt の位置
```

まで戻せる可能性がある。MoonBitがcall site locationの自動注入を持っているのは、この用途では妙に刺さる。 ([MoonBit Documentation](https://docs.moonbitlang.com/en/latest/language/fundamentals.html?utm_source=chatgpt.com))

---

# 実はMoonBitより「乗る」思想に近い候補がStarlark

これも結構気になる。

Starlarkはそもそも**他のアプリケーションへ埋め込んで設定・スクリプティングを提供するための小さな言語**で、仕様上deterministic / hermeticを強く意識している。 ([GitHub](https://github.com/bazelbuild/starlark/blob/master/spec.md?utm_source=chatgpt.com))

さらにMetaの`starlark-rust`があり、

- Rustへのembedding
- Rust型とのinterop
- parser/AST
- linter
- LSP

まで既に提供している。GPUI/Rustで作るLatticeとはかなり都合がいい。 ([GitHub](https://github.com/facebook/starlark-rust?utm_source=chatgpt.com))

つまり、

```text
GPUI/Rust
   +
starlark-rust
   +
Lattice domain API
```

だけで済む。

これは実装コストだけ見れば強敵。

例えば、

```python
game = media("capture.mp4")

scene(
    "demo",
    over = narration.demo,
    body = lambda s: [
        ...
    ],
)
```

みたいな世界。

ただし本家Starlarkは動的型付けなので、我々が面白がっていた、

```text
SourceTime != SceneTime
Region<SourceSpace> != Region<CanvasSpace>
```

を静的型として表す魅力はかなり落ちる。Rust実装にはoptionalなruntime type checkingなどの拡張はあるが、MoonBit/独自VELの型システムとは性質が違う。 ([GitHub](https://github.com/facebook/starlark-rust?utm_source=chatgpt.com))

だから、

**実装コスト最優先ならStarlark、型とプログラミング言語としての気持ちよさならMoonBit**

という感じ。

---

# Rustに直接乗る手もある

これも忘れてはいけない。

Rustにはtokenを扱うproc macroがあり、構文拡張を作れる。 ([Rust言語のドキュメント](https://doc.rust-lang.org/reference/macros.html?utm_source=chatgpt.com))

なので、

```rust
lattice! {
    scene demo over narration.demo {
        source gameplay = game[12m35s..12m43s];

        canvas {
            place gameplay.video {
                fill = canvas;
            }
        }
    }
}
```

みたいにすれば、**かなりVELそのものに近いsyntaxをRustの中へ押し込める**。

しかもLattice本体もRust。

魅力はある。

でも私はこれはあまり好きじゃない。

動画一本書くたびに、

```text
Cargo
crate
Rust compiler
proc macro
```

を意識させるのは重いし、Agentは喜んでも人間の動画編集記述としては「Rustを書いている感」が強すぎる。

---

# そして、私が一番面白いと思う折衷案

これ。

```text
             Lattice Core
             Project Graph
                  ▲
        ┌─────────┴─────────┐
        │                   │
      .vel                 MoonBit
  declarative DSL      programmable layer
        │                   │
        └─────────┬─────────┘
                  ↓
             Project Graph
```

つまり**VELは捨てない**。

でもVELを汎用言語に育てようともしない。

VEL：

```vel
scene demo {
  game[10s..20s]

  text "Hello" at 2s for 3s
}
```

普通の編集はこっち。

ところが、

```text
複雑な素材selection
独自algorithm
数理的animation
大量生成
特殊なarrangement
```

になったら、

```vel
use "./montage.mbt" as montage
```

のようにMoonBit componentを呼ぶ。

MoonBit側は、

```moonbit
pub fn generate(ctx : LatticeContext) -> Composition {
  ...
}
```

をexport。

---

# MoonBitを「Lattice plugin language」にする

これ、かなり相性がいいと思う。

MoonBitはWasm Component ModelをWIT経由で扱えるので、例えばLatticeが、

```wit
interface lattice {
    record source-id { ... }
    record span { ... }

    place: func(source: source-id, span: span) -> placement;
}
```

みたいなPlugin ABIを提供する。

MoonBit plugin：

```text
.mbt
 ↓
Wasm Component
 ↓
Lattice
```

。

Lattice CoreはRust。

Pluginはsandbox可能。

モデル内蔵も不要。

しかも将来MoonBit以外からもWITを実装できる。MoonBit公式にもComponent Model/WITのコード生成経路がある。 ([MoonBit Documentation](https://docs.moonbitlang.com/ja/latest/toolchain/wasm/component-model-tutorial.html?utm_source=chatgpt.com))

これは相当美しい。

---

## なので今の順位

私ならこう見る。

| 案 | 評価 | 理由 |
|---|---:|---|
| **独自VEL + MoonBit extension** | **◎** | DSLの気持ちよさと汎用性を分離 |
| MoonBitそのものをVEL化 | ○ | 言語処理系コスト激減、型が強い |
| Starlarkベース | ○ | Rust embeddingが非常に楽、決定論的 |
| Rust DSL / proc macro | △ | 強いが動画記述として重い |
| 完全独自VELを汎用言語化 | △ | 楽しいが、本業がコンパイラになる |

特に今まで、

```vel
each clip in montage { ... }
Collection<T>
where(...)
arrange(...)
```

とか言い始めたでしょう。

**この辺から黄色信号なのよ。**

VELに、

```text
generic
collection algebra
control flow
user-defined function
module
package
trait
```

まで欲しくなった瞬間、

> 「それMoonBitでよくない？」

がどんどん強くなる。

逆に、

```vel
scene
source
canvas
mix
speech
arrange
curve
```

くらいでVELを止められるなら、独自DSLの価値がかなり残る。

だから私は今、**VELには「汎用計算能力を追加しない」という明確な境界を置いて、脱出ハッチとしてMoonBitを使う**案にかなり傾くわ。

これなら「動画編集言語を作る」という面白さは残しながら、ある日VELに赤黒木やHTTP clientを実装し始める悲劇を回避できる。

---

# you asked

message time: 2026-08-19 01:30:18

エクステンションに逃がせることを色々考える

---

# chatgpt response

ここはかなり大事ね。**「VELを小さく保つために何をExtensionへ追い出すか」だけでなく、Extensionがどの段階へ介入できるかを先に分類した方がいい**と思う。そうすると、何でもプラグイン化してCoreが空洞になるのを防げるわ。

そうね。ここでかなり大胆に**「Coreにある必要がないもの」を追い出してみる**と、VELを小さく保てそうだわ。

## 事実

Extensionには少なくとも、介入する段階が違うものがある。

```text
Surface VEL
    ↓
Compiler / Core Graph
    ↓
Resolve
    ↓
Render Plan
    ↓
Backend

      ＋

Lattice Studio / CLI
```

だから「Extension」という一種類の万能プラグインにせず、

```text
Compile extension
Resolve extension
Render extension
Tooling extension
```

くらいに分けた方が、責務がかなり明確になる。

---

# 推測（示唆）

## まず、かなりExtensionへ逃がせる

### 1. 解析系

これはほぼ全部追放していいと思う。

```text
beat detection
scene detection
silence detection
motion analysis
shot boundary
ASR
speaker diarization
object tracking
face detection
OCR
VLM analysis
semantic search
```

Coreが知るのは結果の型だけ。

```text
EventSet<T>
SpanSet<T>
Signal<T>
RegionTrack<T>
```

例えば、

```vel
events beats =
  music |> beats()
```

の`beats()`はExtension。

Coreから見れば、

```text
Audio
  ↓ opaque extension invocation
EventSet<Beat>
```

にすぎない。

これはすごく綺麗。

---

# 2. TTSエンジン

これも**Speech自体はCore、Engineは全部Extension**がいい。

```text
Core
Speech
Utterance
WordTiming
PhonemeTiming
Voice
SpeechMark
```

だけ定義。

そこに、

```text
VOICEVOX
AivisSpeech
ElevenLabs
OpenAI
Style-Bert-VITS2
ローカル独自TTS
```

を差す。

例えば、

```vel
speech_engine narrator =
  extension("voicevox").engine {
    speaker = 3
  }
```

あるいはSurface Sugarで、

```vel
speech_engine narrator = voicevox {
    speaker = 3
}
```

。

でも`voicevox`という単語をCompilerが知っている必要はない。

これはかなり重要ね。

**「音声合成をnativeに扱う」と「VOICEVOX対応をCoreに書く」は別物。**

---

# 3. Effect

これも大部分を追い出せる。

Coreに必要なのは、

```text
Property<T>
Curve<T>
Visual
Audio
Region
Transform
```

と、

```text
Effect<Input, Output>
```

くらい。

具体的な、

```text
Gaussian Blur
Glow
Sharpen
Film Grain
Chromatic Aberration
LUT
Vignette
Compressor
Limiter
Reverb
Noise Reduction
```

はExtension/stdlib。

例えば、

```vel
source dramatic =
  game
  |> blur(radius: 8px)
  |> glow(amount: 0.2)
```

でも`blur`と`glow`は登録されたoperator。

VELのparserには何も追加しない。

これならEffect ecosystemをCompilerから切り離せる。

---

# 4. Transition

同じく、

```text
cut
crossfade
```

くらいをstdlib扱いにして、

```text
wipe
zoom transition
glitch
light leak
morph
```

全部Extensionでいい。

むしろCoreでは、

```text
Transition {
    left
    right
    duration
    parameters
}
```

という意味しか必要ないかもしれない。

---

# 5. Layout algorithm

これも面白い。

`layout`という概念はCoreでも、

```vel
layout horizontal { ... }
```

の`horizontal`までCompiler組み込みである必要はない。

```text
horizontal
vertical
grid
picture-in-picture
golden-ratio
masonry
comic-panel
```

はLayout Extension。

つまり、

```text
layout(inputs, constraints)
    ↓
Collection<SpatialPlacement>
```

というcontractだけCoreに置く。

時間方向でも同様。

---

# 6. `arrange()`のstrategy

これはExtension向きだわ。

Core：

```text
Collection<Source>
+
SpanSet
       ↓
Arrangement
       ↓
Collection<Placement>
```

。

strategy：

```text
sequential
cycle
random-with-seed
best-fit
beat-match
duration-fit
alternating
montage
```

はExtension。

例えば、

```vel
shots
|> arrange(
     into: slots,
     strategy: beat_match()
   )
```

。

`arrange`という基本演算だけVELが知っていて、**どう割り当てるかをextensionへ委譲**できる。

これはかなり強い。

---

# 7. Generator

これも大量に逃がせる。

```text
text
shape
gradient
noise
subtitle card
lower third
waveform visualization
spectrum
progress bar
map
chart
QR code
code snippet renderer
Markdown renderer
```

。

例えば、

```vel
source title =
  lower_third(
    title: "Prisma Obex",
    subtitle: "Finisher Build"
  )
```

。

`lower_third`はGenerator Extension。

Canvasには単なるVisualとして出てくる。

---

# 8. Import / Export

ここもCoreをかなり薄くできる。

### Import

```text
Premiere XML
FCPXML
EDL
ASS/SRT
OTIO
Audacity labels
```

↓

```text
Project Graph / EventSet / Caption etc.
```

。

### Export

```text
YouTube
YouTube Shorts
ProRes master
GIF
HLS
EDL
FCPXML
OTIO
```

。

ただし、

```vel
export youtube { ... }
```

自体はPreset Extensionでもいい。

Coreは、

```text
RenderTarget
VideoFormat
AudioFormat
```

だけ。

---

# 9. Codec / Backend

もっと下まで行けば、

```text
FFmpeg backend
wgpu renderer
GStreamer
AVFoundation
Media Foundation
NVENC
VideoToolbox
VAAPI
```

も理論上Extension。

ただしこの辺は性能・GPU handle・巨大bufferが絡むので、MoonBit/Wasm Extensionとは別にした方がいい。

私は、

```text
Extension
├ Wasm Component Extension
└ Native Driver
```

と分けたい。

### Wasm Extension

MoonBit等。

```text
analysis
arrangement
generator
lint
import/export
project transformation
```

に向く。

### Native Driver

Rust/C/C++。

```text
decode
encode
GPU render
audio device
hardware acceleration
native TTS runtime
```

。

ここを同じplugin ABIで無理やり統一すると、多分後悔するわ。

---

# 10. Linter

これは全部Extension可能にしていい。

Core compilerは、

```text
syntax
typing
graph invariant
time/space safety
```

だけ責任を持つ。

それ以上の、

```text
字幕がsafe area外
切り替えが速すぎる
絶対時間literalが多い
同じcropが繰り返されている
音量差が大きい
YouTube Shortsでは文字が小さい
```

はLint Extension。

プロジェクト単位で、

```toml
[lint]
plugins = [
  "lattice/std-video",
  "lattice/youtube",
]
```

みたいにできる。

これならゲーム実況特化lintなんてものも作れる。

```text
ゲームHUDと字幕が重なっている
facecam領域へ字幕が入っている
```

とか。

---

# 11. Refactoring

これもExtensionにできる。

例えば、

```text
Extract Scene
Extract Source
Extract Composition
Convert to Pipeline
Collapse shorthand
Extract repeated effect
Turn manual placements into arrange()
```

。

APIとして、

```text
AST / Project Graph
    ↓
Refactor
    ↓
SourceEdit[]
```

があればいい。

そしてゲーム実況用Extensionが、

> 「この6個の連続CutをMontage Sceneへ抽出」

とかやってもいい。

---

# 12. Studio UI

これも結構追い出せる。

Core Studio：

```text
Code
Timeline
Canvas
Structure
Inspector
Problems
```

。

Extension：

```text
Beat editor
Color grading panel
Subtitle editor
Audio mixer
Speech editor
Motion tracker panel
Scopes
Waveform tool
Asset browser provider
```

。

GPUIなら最終的に、

```text
Panel extension
Inspector extension
Canvas tool
Timeline overlay
Command
```

くらいのUI extension pointはありそう。

ただし、これを最初からやるとGPUIプラグインホスト開発で半年死ぬので、v1では後回しでいい。

---

# 13. CLI command

地味にこれも欲しい。

Extensionが、

```bash
lattice voicevox ...
lattice subtitles ...
lattice youtube ...
```

のsubcommandを提供できる。

ただ個人的には、

```bash
lattice ext voicevox ...
```

みたいにnamespaceを切った方が衝突しづらい。

---

# 逆にCoreから絶対追い出したくないもの

ここがもっと重要。

私はこれだけはLatticeの憲法として保持したい。

```text
Time
Duration
TimeMap

CoordinateSpace
Region
Geometry

Media / Stream

Sequence
Scene
Placement
Composition

Property<T>
Curve<T>

EventSet<T>
SpanSet<T>
Signal<T>
Collection<T>

Project Graph

EditPatch

Diagnostics

Extension Invocation / Provenance
```

要するに、

> **時間・空間・配置・値・依存関係**

はCore。

それ以外の「何をするか」はかなりExtensionへ逃がせる。

---

# 特にSyntax Extensionは許したくない

ここは私はかなり強く反対する。

例えばMoonBit pluginが、

```vel
magical_montage! {
   ...
}
```

みたいな独自VEL grammarを追加できるようにはしない。

なぜなら一気に、

```text
parser
formatter
LSP
highlight
code ↔ GUI mapping
refactoring
```

がExtension dependentになる。

これはLatticeのIDE思想と相性が悪すぎる。

代わりに固定grammarの中で、

```vel
use "cool-montage"

placements x =
  shots
  |> cool_montage(
       beats: beats,
       aggression: 0.8
     )
```

と書かせる。

**Extensionは新しい語彙を追加できるが、新しい文法は追加できない。**

この線引きはかなり重要だと思う。

---

# Project Graph NodeをExtensionに許すか

これも危険なところ。

例えばExtensionが、

```text
MyMagicNode
```

というLattice Coreの知らないNodeを勝手にProject Graphへ追加できるようにすると、

```text
Timelineはどう表示する？
Git diffはどう解釈する？
Inspectorは？
Lintは？
別環境でpluginが無かったら？
```

となる。

なので私は原則、

> **ExtensionはCore Graphへlowerできなければならない**

を推したい。

例えば、

```text
FancyMontage
     ↓ extension evaluation
Core Placement[]
```

。

```text
FancyLowerThird
     ↓
Core Composition
```

。

そうするとExtensionがなくても、`lattice.lock`にresolved graphを保存すれば再現できる可能性がある。

---

ただしEffectだけは難しい。

例えば独自shaderは、

```text
Visual
 ↓
Custom GPU Effect
 ↓
Visual
```

なのでCore primitiveへ分解できない場合がある。

そこだけ、

```text
OpaqueEffect {
    extension_id
    version
    params
}
```

を許す。

つまり**Opaque Nodeはescape hatch中のescape hatch**。

---

# Pure / Impureも区別したい

Extension manifestで、

```text
determinism = pure
```

とか欲しい。

例えば、

```text
crop implementation
layout
arrangement with fixed seed
```

はPure。

TTSは、

```text
resolved
```

。

ネットワークへアクセスするVLMは、

```text
external
```

。

例えば、

```text
PURE
input → output
常に同じ

RESOLVABLE
input → artifact
lockすれば固定

LIVE
外部状態に依存
```

の3分類。

そしてCompiler/IDE上で見える。

```text
✓ pure
◇ resolved
⚠ live
```

。

前に話していた決定論性が、ここでExtension contractとして自然に戻ってくる。

---

# Extension manifest

かなり先の話だけれど、例えば、

```toml
id = "io.lattice.voicevox"
version = "1.2.0"

runtime = "wasm-component"

[capabilities]
network = false
filesystem = false

[provides]
speech_engines = ["voicevox"]
```

とか。

Beat detectorなら、

```toml
id = "io.lattice.audio.beats"

[provides]
operators = ["beats"]
```

。

MoonBitで書いてWasm Componentへcompile。

ここは前の案とかなり綺麗に繋がる。

---

# こうするとVELのstdlibすらExtensionとして考えられる

これが一番面白いかもしれない。

Compiler Core：

```text
Time
Space
Graph
Types
Extension ABI
```

だけ。

公式stdlib：

```text
lattice/std-media
lattice/std-canvas
lattice/std-audio
lattice/std-layout
lattice/std-effects
lattice/std-speech
lattice/std-analysis
```

。

ユーザーからは全部最初から入っているので違いを感じない。

でも設計上は、

```text
Compiler ≠ Standard Library
```

になる。

これは汎用言語と同じね。

---

## かなり小さなLattice Coreまで削れる

最終的には、

```text
                Lattice Core

       Time / Space / Type System
                 │
          Project Graph
                 │
       Evaluation / Lowering
                 │
       Extension Contracts
                 │
      Diagnostics / Provenance
```

だけでも成立するかもしれない。

そして、

```text
        Official Extensions
─────────────────────────────────
Media      FFmpeg
Canvas     basic composition
Audio      mixer
Speech     speech model
Effects    basic transforms
Analysis   beats/silence
Export     common formats

        Third Party
─────────────────────────────────
VOICEVOX
Aivis
special transitions
game-specific overlay
automatic montage
YouTube workflow
etc.
```

。

こうすると、**「これはVELに入れるべきか？」という議論のかなりの部分を「Core semantic primitiveか、それともExtension operationか？」に置き換えられる**。

私はこの境界をかなり強くしたい。

特に今後、新機能を思いついたときは、

> **それが無いとProject Graphそのものを意味づけられないか？**

だけ問う。

YESならCore候補。

NOなら、まずExtensionへ追放。

この原則ならVELがMoonBit級の汎用言語へ肥大化するのも、Lattice Coreが動画編集業界の全機能を抱え込むのも、かなり防げそうよ。

---

# you asked

message time: 2026-08-19 01:37:17

うん、とにかくmoonbit側に追放していきたいね。
ここで改めて、スタックを整理しよう

---

# chatgpt response

そうね。ここまでの話を全部いったん畳むと、かなり綺麗な「細いCore＋太いExtension」構成になってきたわ。特に **VELは編集意味論だけ、計算・解析・生成・賢い処理はMoonBitへ追放** を中心原則に置く。

うん。今のLatticeは、かなり明快に **「Rustで土台、VELで記述、MoonBitで拡張、GPUIで触る」** に整理できると思うわ。

## 事実

ここまで決めた・かなり強く合意できているものを積むと、こうなる。

```text
                         Lattice

┌──────────────────────────────────────────────────────┐
│                    Human / Agent                     │
│                                                      │
│   Lattice Studio        Coding Agent                 │
│      GPUI                  │                         │
│        │                   │ edit / shell            │
│        │                   ▼                         │
│        │              lattice CLI                    │
│        │                   │                         │
│        │              MCP adapter (later)            │
├────────┴───────────────────┴─────────────────────────┤
│                     VEL Layer                        │
│                                                      │
│  *.vel → Parser → Desugar → Core VEL → Typecheck    │
│                                   ↓                  │
│                            Project Graph             │
├──────────────────────────────────────────────────────┤
│                   Lattice Core / Rust                │
│                                                      │
│ Time / Space / Media / Scene / Placement / Property │
│ Composition / Event / Span / Signal / Diagnostics   │
│ Extension ABI / Resolve / Render Planning           │
├───────────────────┬──────────────────────────────────┤
│ MoonBit Extensions│ Native Rust Drivers              │
│                   │                                  │
│ analysis          │ media decode / encode            │
│ arrangement       │ FFmpeg integration               │
│ generators        │ GPU / OS integration             │
│ TTS adapters      │ filesystem                       │
│ lint              │ high-throughput processing       │
│ import/export     │                                  │
│ project transforms│                                  │
│        │          │                                  │
│   Wasm Component? │                                  │
├────────┴──────────┴──────────────────────────────────┤
│               Media / Render Backend                 │
│                                                      │
│        decode → process → composite → mix → encode   │
│                       FFmpeg first                   │
├──────────────────────────────────────────────────────┤
│                    Workspace                         │
│                                                      │
│ *.vel / lattice.toml / lattice.lock / Git            │
│ external media / .lattice cache / exports            │
└──────────────────────────────────────────────────────┘
```

私はこの構造を基本形にしたい。

---

## 各層の責務

| 層 | 技術 | やること |
|---|---|---|
| **Studio** | Rust + GPUI | Code / Canvas / Timeline / Structure / Inspector |
| **Automation** | Rust CLI | check / inspect / query / render / resolve |
| **Agent bridge** | CLI → MCP | 外付けCoding Agentから操作 |
| **VEL Surface** | 独自DSL | 人間が動画編集を記述 |
| **VEL Compiler** | Rust | parse / desugar / typecheck / lint |
| **Lattice Core** | Rust | 動画編集の基本意味論 |
| **Extension runtime** | MoonBit中心 | 高度な処理・アルゴリズム |
| **Native drivers** | Rust | 重いI/O・GPU・codec・OS |
| **Render** | FFmpeg first | 実際の動画・音声生成 |
| **Workspace** | filesystem + Git | ソース、config、lock、履歴 |

ここで一番重要なのは、**MoonBitとRustの境界**ね。

---

# Lattice Coreはかなり小さくする

Coreには、「動画編集というものを成立させるため絶対必要な意味」しか入れない。

```text
Time
Duration
TimeMap

SourceTime
SceneTime
SequenceTime

CoordinateSpace
Region
Geometry

Media
Stream

Sequence
Scene
Placement
Composition

Canvas
Mix

Property<T>
Curve<T>

EventSet<T>
SpanSet<T>
Signal<T>
Collection<T>

Project Graph
Diagnostics
Provenance

Extension Invocation
```

このくらい。

例えば、

```text
Beat Detection
```

は動画編集そのものではない。

だからCoreには入れない。

でも、

```text
EventSet<Beat>
```

という**Beat Detectionの結果を置ける場所**はCoreに必要。

この違いをかなり厳密にしたいわ。

---

# VELも小さくする

VEL自身が知るべき概念もかなり限定する。

例えば、

```vel
sequence main flow {
  use intro
  use demonstration
  use conclusion
}

scene demonstration {
  source game = capture[10s..20s]

  canvas {
    place game.video {
      fill = canvas
    }
  }

  mix {
    place game.audio
  }
}
```

くらい。

VELに、

```text
高度な画像認識
Beat解析アルゴリズム
TTS実装
Montage selection algorithm
Neural denoise
HTTP client
任意のデータ処理
```

を実装させない。

VELは**Project Graphを組み立てる言語**に徹する。

---

# MoonBitは「VELから逃げる場所」

ここをかなり積極的に使う。

例えば、

```vel
events beats =
  song.audio
  |> beats()
```

というVELを書く。

`beats()`そのものはMoonBit Extension。

同様に、

```vel
shots
|> arrange(
     into: slots,
     strategy: energetic_montage()
   )
```

なら、

```text
energetic_montage()
```

もMoonBit。

TTSなら、

```vel
speech_engine narrator =
  voicevox(...)
```

のVOICEVOX adapterがMoonBit。

Lintなら、

```text
YouTube Shorts用caption readability
ゲームHUDとの衝突
Montageのtempo consistency
```

なんかもMoonBit。

つまり、

```text
「これVELに言語機能として追加しようかな？」
```

と思ったら、まず、

```text
MoonBitの関数として書けないか？
```

を考える。

**書けるなら追放。**

これくらいでいいと思う。

---

# ただしMoonBitにも追放しないものがある

性能やOS資源に直結する部分。

例えば、

```text
動画decode
動画encode
GPU texture
zero-copy frame
audio device
filesystem watcher
FFmpeg
hardware codec
```

。

これはMoonBit/Wasmへ押し込んでも得しない可能性が高い。

だから、

```text
Domain intelligence → MoonBit
Machine plumbing    → Rust
```

くらいの分業が綺麗。

言い換えると、

```text
「どう編集する？」
       MoonBit

「実際どう処理する？」
       Rust
```

ね。

---

# Extension ABI

ここはまだ**候補**だけれど、

```text
MoonBit source
    ↓
Wasm Component
    ↓
Lattice Extension Host
```

はかなり有力。

例えばLatticeが、

```text
Input
  Audio
  Collection<Source>
  EventSet
  SpanSet
  Project query

Output
  EventSet
  SpanSet
  Placement[]
  Composition
  Diagnostics
```

をextensionへ渡す。

MoonBit側は、

```text
input → core-compatible output
```

だけ返す。

重要なのは原則として、

> **Extensionは最終的にCore Graphへlowerする**

こと。

例えばMontage extensionなら、

```text
FancyMontage
    ↓ resolve
Placement[]
```

。

独自の永続的な`FancyMontageNode`をProject Graphへ住まわせない。

そうすればExtensionが増えてもCoreは腐らない。

---

# Effectだけはescape hatchが必要

全部Coreへlowerできるわけではない。

例えば、

```text
独自shader
特殊なaudio processor
```

は分解できないかもしれない。

なので、

```text
OpaqueEffect {
  extension
  version
  params
}
```

程度は許す。

ただしこれは**例外**。

何でもOpaqueにしたら、VELがXML wrapperになるからね。

---

# VEL Compiler

Rust側には結構しっかりしたCompilerを持つ。

```text
Surface VEL
    ↓
CST / AST
    ↓
Desugar
    ↓
Core VEL
    ↓
Name Resolution
    ↓
Type / Unit Check
    ↓
Project Graph
    ↓
Semantic Lint
```

ここまでは純粋。

そのあと、

```text
Project Graph
    ↓
Resolve Extensions
    ↓
Resolved Project
    ↓
Render Planning
```

。

つまり重要な原則として、

> **Compile != Resolve != Render**

にする。

TTS APIを叩くのはCompilerじゃない。

Beat解析を走らせるのもCompilerじゃない。

これなら`lattice check`は高速で決定論的にできる。

---

# Render stack

最初はあまり野心を出さなくていい。

```text
Lattice Render Plan
        ↓
FFmpeg adapter
        ↓
FFmpeg
```

。

Previewも最初は、

```text
FFmpeg decode
↓
frame
↓
GPUI texture
↓
Canvas overlays
```

あたりでいい。

後から、

```text
wgpu/native compositor
```

が欲しくなれば増やす。

VELやProject GraphをFFmpeg仕様に合わせないことだけ守る。

---

# Studio

GPUI側はLattice Coreのprojection。

```text
Project Graph
├─ Code View
├─ Structure View
├─ Timeline
├─ Canvas
├─ Inspector
└─ Problems
```

GPUI依存はCoreへ漏らさない。

だから、

```text
lattice-core
```

はGUIなしでも、

```bash
lattice check
lattice render
```

で完全に使える。

Studioは**強いフロントエンドの一つ**に過ぎない。

---

# CLI

こっちは逆にかなり重要。

```bash
lattice check
lattice fmt

lattice inspect scene demo
lattice query ...

lattice resolve

lattice render frame ...
lattice render scene ...
lattice render project ...
```

ほぼ全部、

```bash
--json
```

対応。

これがCoding Agent APIにもなる。

---

# Agent integration

Lattice本体には、

```text
OpenAI SDK
Claude SDK
Gemini SDK
xAI SDK
agent runtime
prompt manager
model router
```

**全部なし。**

Coding Agentが外から、

```text
read VEL
↓
edit
↓
lattice check
↓
lattice render
↓
inspect result
```

をやる。

必要になったら、

```text
lattice mcp
```

をCLI/Core APIの薄いadapterとして足す。

この境界はかなり固定していいと思う。

---

# Workspace

Projectという大層なDBも作らない。

```text
my-video/
├─ lattice.toml
├─ lattice.lock
├─ main.vel
├─ scenes/
│  └─ ...
├─ assets/
│  └─ small stuff
├─ .lattice/
│  ├─ cache/
│  ├─ proxy/
│  └─ index/
└─ exports/
```

巨大Mediaは外部参照で構わない。

`.lattice/`は全部消して再構築可能。

履歴はGit。

Undo/RedoだけStudio session内のEditPatch stack。

---

# `lattice.lock`

ここはExtension化が進むほど重要になりそう。

```text
VEL
    「beats()して」
        ↓
Extension
        ↓
EventSet<Beat>
```

や、

```text
Speech text
    ↓
TTS extension
    ↓
audio + timings
```

の結果を必要なら固定する。

つまり、

```text
*.vel
    意図

lattice.lock
    Resolveされた外部結果・version・hash
```

。

これで「賢いExtensionを大量に使いつつ、最終Projectは固定できる」が成立する。

---

## 推測（示唆）

ここまで整理すると、Latticeは実は**三つの製品を一つに見せている**と考えると分かりやすい。

```text
VEL
    動画編集の記述言語

Lattice Core
    動画編集用compiler/build system

Lattice Studio
    Project Graph用IDE
```

そしてMoonBitは第四の、

```text
Lattice SDK
    ユーザーが編集能力を増やすための言語
```

になる。

これがかなりいい。

特に私は、MoonBitを単なる「pluginを書く選択肢」より、

> **VELが汎用言語へ肥大化しないための公式escape hatch**

として位置付けたい。

そうすると新機能を考えるとき、

```text
これは時間・空間・配置の基本意味か？
       ↓ YES
      Core

       ↓ NO

VELで直接記述する基本操作か？
       ↓ YES
      stdlib / Surface VEL

       ↓ NO

MoonBitで実装できるか？
       ↓ YES
    Extension

       ↓ NO / performance or OS
      Rust native driver
```

という判断木ができる。

このルール、かなり強いと思う。

**Rustは機械を動かす。VELは動画を記述する。MoonBitは賢いことをする。GPUIは人間に見せる。CLIは外界につなぐ。Gitは歴史を持つ。**

今のLatticeのスタックは、そこまで一行で整理できるところまで来たわ。

---

# you asked

message time: 2026-08-19 08:12:31

拡張を書いてみる

---

# chatgpt response

いいわ。最初の一本は、外部APIやGPUに逃げず、**MoonBitだけで完結する「Arrangement拡張」**がちょうどいい。これならExtension ABI・型・determinism・Coreへのloweringを一度に試せるわ。

いいわ。最初の一本は **`energetic-montage` Arrangement Extension** にしよう。高度すぎず、それでも「MoonBitへ追放する意味」がちゃんとある。

MoonBitは現在WITから`wit-bindgen moonbit`でComponent Model用bindingを生成できるので、**Lattice ↔ Extensionの境界をWIT、アルゴリズム本体をMoonBit**という形は実際に取れる。WIT側はrecord/list/variantなどを定義できる。([MoonBit Documentation](https://docs.moonbitlang.com/en/latest/toolchain/wasm/component-model-tutorial.html?utm_source=chatgpt.com))

## 事実

やらせたいのはこれ。

```vel
placements montage =
  shots
  |> arrange(
       into: slots,
       strategy: energetic_montage(
         alternate: true
       )
     )
```

Extensionへの入力は、

```text
Sources
├ id
├ duration
├ energy
└ tags

Slots
├ start
├ duration
└ strength
```

。

出力はただのCore型、

```text
Placement[]
```

にする。

つまりExtension独自の`EnergeticMontageNode`はProject Graphに残らない。

---

## 1. WIT

最初のABIなら、思ったより小さくできる。

```wit
package lattice:arrangement@0.1.0;

interface arrangement {
    record rational {
        num: s64,
        den: u64,
    }

    record source {
        id: string,
        duration: rational,

        // 0.0 .. 1.0
        energy: f32,

        tags: list<string>,
    }

    record slot {
        index: u32,

        start: rational,
        duration: rational,

        // beat strengthなど。
        // 0.0 .. 1.0
        strength: f32,
    }

    record request {
        sources: list<source>,
        slots: list<slot>,

        // deterministic tie-breaking用。
        seed: u64,
    }

    record placement {
        source-id: string,
        slot-index: u32,

        // Source内のどこから使うか。
        source-start: rational,

        duration: rational,
    }

    variant arrange-error {
        no-sources,
        source-too-short(string),
        invalid-time,
    }

    arrange: func(input: request)
        -> result<list<placement>, arrange-error>;
}

world energetic-montage {
    export arrangement;
}
```

この境界がかなり重要ね。

MoonBit側には、

```text
Canvas
GPUI
VEL AST
FFmpeg
Project DB
```

を一切見せていない。

**「素材集合を時間スロットへ割り当てろ」だけ。**

---

# 2. MoonBit側の純粋なロジック

WIT glueを除けば、本体はこういう程度でいい。

```moonbit
struct Candidate {
  id : String
  duration_num : Int64
  duration_den : UInt64
  energy : Float
  tags : Array[String]
}

struct Slot {
  index : UInt
  duration_num : Int64
  duration_den : UInt64
  strength : Float
}

struct Choice {
  source_id : String
  slot_index : UInt
}

fn score(
  source : Candidate,
  slot : Slot,
  previous : Candidate?,
) -> Double {
  let energy_distance =
    (source.energy.to_double() - slot.strength.to_double()).abs()

  let energy_score =
    1.0 - energy_distance

  let repeat_penalty = match previous {
    Some(prev) =>
      if prev.id == source.id {
        0.35
      } else {
        0.0
      }

    None => 0.0
  }

  energy_score - repeat_penalty
}
```

MoonBitには普通のstruct、Array、pattern matching、数値型があるので、この種のロジックは完全に普通のアプリケーションコードとして書ける。([MoonBit Documentation](https://docs.moonbitlang.com/en/latest/language/fundamentals.html?utm_source=chatgpt.com))

そして選択。

```moonbit
fn choose_source(
  sources : Array[Candidate],
  slot : Slot,
  previous : Candidate?,
) -> Candidate? {
  let mut best : Candidate? = None
  let mut best_score = -1000000.0

  for source in sources {
    let current = score(source, slot, previous)

    if current > best_score {
      best = Some(source)
      best_score = current
    }
  }

  best
}
```

Montage全体。

```moonbit
fn arrange(
  sources : Array[Candidate],
  slots : Array[Slot],
) -> Array[Choice] {
  let result : Array[Choice] = []

  let mut previous : Candidate? = None

  for slot in slots {
    match choose_source(sources, slot, previous) {
      Some(source) => {
        result.push({
          source_id: source.id,
          slot_index: slot.index,
        })

        previous = Some(source)
      }

      None => ()
    }
  }

  result
}
```

これだけでも、

```text
強いbeat → energyの高い素材
弱いbeat → energyの低い素材
同じ素材連発 → penalty
```

という編集戦略になる。

---

# 3. もう少し編集っぽくする

タグも使いたい。

例えば強拍では`impact`を優先。

```moonbit
fn has_tag(source : Candidate, wanted : String) -> Bool {
  for tag in source.tags {
    if tag == wanted {
      return true
    }
  }

  false
}

fn score(
  source : Candidate,
  slot : Slot,
  previous : Candidate?,
) -> Double {
  let energy_distance =
    (source.energy.to_double() - slot.strength.to_double()).abs()

  let mut result =
    1.0 - energy_distance

  if slot.strength > 0.85 &&
     has_tag(source, "impact") {
    result = result + 0.4
  }

  if slot.strength < 0.4 &&
     has_tag(source, "movement") {
    result = result + 0.2
  }

  match previous {
    Some(prev) =>
      if prev.id == source.id {
        result = result - 0.35
      }

    None => ()
  }

  result
}
```

VELにこのロジックを入れ始めると、

```vel
when beat.strength > ...
prefer ...
penalty ...
```

と独自のルール言語が増殖していた。

MoonBitへ追放すると、

**ただの関数。**

ここがまさに狙っていたところよ。

---

# 4. VEL側

ユーザーから見えるのは非常に小さくできる。

```vel
use extension "lattice/energetic-montage" as energetic

scene montage {
  source music =
    bgm[42s..58s]

  events beats =
    music.audio
    |> beats()

  spans slots =
    beats
    |> between()

  sources shots = [
    shot_a {
      energy = 0.2
      tags = ["movement"]
    },

    shot_b {
      energy = 0.9
      tags = ["combat", "impact"]
    },

    shot_c {
      energy = 0.6
      tags = ["combat"]
    },

    shot_d {
      energy = 1.0
      tags = ["impact"]
    },
  ]

  placements montage =
    energetic.arrange(
      sources: shots,
      slots: slots,
      seed: 42,
    )

  canvas {
    place montage {
      fill = canvas
    }
  }

  mix {
    place music.audio
  }
}
```

ここは面白くて、もはやVEL組み込みの、

```vel
|> arrange(strategy: ...)
```

すらいらない可能性がある。

単純に、

```vel
placements montage =
    energetic.arrange(...)
```

でいい。

---

# 5. Extension manifest

こんなものを横に置く。

```toml
id = "lattice/energetic-montage"
version = "0.1.0"

runtime = "wasm-component"
world = "energetic-montage"

determinism = "pure"

[provides]
arrangements = ["energetic"]

[permissions]
filesystem = false
network = false
clock = false
random = false
```

このExtensionには外界を一切見せない。

WebAssembly Componentの`world`は、何をimport/exportするか自体をinterfaceとして記述できるので、こうしたcapability-orientedな境界とも相性がいい。([Bytecode Alliance Component Model](https://component-model.bytecodealliance.org/design/worlds.html?utm_source=chatgpt.com))

したがって、

```text
✓ Pure

same input
+
same extension version

→ same Placement[]
```

とLatticeが保証できる。

---

# 6. Build

現行のMoonBit公式手順なら、

```bash
wit-bindgen moonbit wit/world.wit \
  --out-dir . \
  --derive-eq \
  --derive-show \
  --derive-error
```

でbinding/project skeletonを生成し、

```bash
moon build --target wasm
```

から`wasm-tools`を使ってComponent化する流れになる。([MoonBit Documentation](https://docs.moonbitlang.com/en/latest/toolchain/wasm/component-model-tutorial.html?utm_source=chatgpt.com))

生成される`stub.mbt`に、

```moonbit
pub fn arrange(input : Request)
  -> Result[Array[Placement], ArrangeError] {
    ...
}
```

相当を実装するイメージね。生成される具体的な型名・package pathは`wit-bindgen`側に任せればいい。

---

## 推測（示唆）

これ一本書くだけで、**Extension ABIについてかなり重要なことが見える**。

### ExtensionはProject Graphを直接触らせなくていい

最初、

```text
extension gets ProjectContext
extension queries scenes
extension edits nodes
```

まで渡そうとしていたけれど、かなり危険だと思う。

このMontageなら、

```text
input value
   ↓
MoonBit
   ↓
output value
```

だけで十分。

つまり基本Extensionは**pure functionに寄せられる**。

```text
Arrangement:
Sources × Spans → Placements

Analyzer:
Audio → EventSet

Layout:
Visuals × Region → SpatialPlacements

Generator:
Params → Visual

Lint:
ProjectView → Diagnostics
```

ものすごく関数型なのよ。

---

### Core APIを巨大なObject APIにしなくて済む

例えばこれは避けたい。

```moonbit
ctx.project().sequence("main")
  .scene("foo")
  .canvas()
  .add(...)
```

これをやるとMoonBit SDKがLattice内部実装と密結合する。

代わりに、

```moonbit
fn arrange(
  sources : Sources,
  spans : Spans,
) -> Placements
```

。

**値を渡して値を返す。**

この方がversioningも圧倒的に楽。

---

### WITのgeneric不足もむしろ設計圧になる

WITのユーザー定義record自体はgenericにできないので、`Collection<T>`をそのままABIへ流すような設計にはできない。([Bytecode Alliance Component Model](https://component-model.bytecodealliance.org/design/wit.html?utm_source=chatgpt.com))

でも私はこれ、悪い制約ではない気がする。

Extension interfaceを、

```text
Arrangement API
Analysis API
Generator API
Lint API
```

のように**用途別の具体的なcontract**へ分ける圧力になるから。

MoonBit内部では好きにgenericを使えばいい。

---

そして何より、

```vel
placements montage =
  energetic.arrange(...)
```

の中身が100行でも1000行でも、**VELは一文字も賢くならなくていい**。

これが狙い通りね。

次に書いてみたいのは逆方向で、`Audio → EventSet<Beat>`を返す**解析Extension**。そっちは音声bufferをExtensionへどう渡すのかという性能境界が出てきて、MoonBit/WasmとRust Native Driverの境界をかなり良くテストできるわ。

---

# you asked

message time: 2026-08-19 08:14:35

もっと原則的なものを逃がしたパターン

---

# chatgpt response

あるわ。むしろ「Montageみたいな高度機能」より、**`flow`や`layout`みたいな一見CoreっぽいものまでMoonBitへ追い出せるか**を考えると、境界が一段はっきりする。私はここで一回、かなり攻めて削ってみたい。

あるわ。もっと攻めるなら、**「Coreは名詞だけ持つ。動詞はMoonBitへ追放する」**くらいまで行けると思う。

## 事実

今までCore候補にしていたものにも、実は二種類ある。

```text
表現そのもの
────────────────
Time
Span
TimeMap
Region
Transform
Placement
Property
Signal
Visual Graph
Audio Graph

その表現を作る方法
────────────────
flow
trim
freeze
speed
crop-to-fit
horizontal layout
crossfade
duck
between
arrange
scene over speech
```

後者はかなりの範囲で、

> **入力されたCore値から、別のCore値を計算する純粋関数**

にできる。

ならMoonBitでいい、という話になるわ。

---

# 1. `flow`すら追放する

例えば、

```vel
sequence main flow {
  use hook
  use intro
  use demo
  use outro
}
```

は一見、言語の根本機能に見える。

でも意味は単に、

```text
hook.start  = 0
intro.start = hook.end
demo.start  = intro.end
outro.start = demo.end
```

でしょう。

ならMoonBitでは、

```moonbit
pub fn flow(
  items : Array[TemporalItem],
  start : Time,
) -> Array[Placement] {
  let mut cursor = start
  let result = []

  for item in items {
    result.push(Placement::{
      item,
      start: cursor,
      duration: item.duration(),
    })

    cursor = cursor + item.duration()
  }

  result
}
```

程度でいい。

VELの、

```vel
sequence main flow { ... }
```

はSurface sugarに過ぎず、

```text
std.flow([...])
```

へdesugarする。

つまり**SequenceはCoreでも、Sequenceの並べ方はCoreではない**。

これはかなり良い境界だと思う。

---

# 2. `clip`さえ追放できる

もっと行く。

今までは、

```vel
clip demo = game[10s..20s]
```

を基本構文だと思っていた。

でもこれが実際やっていることは、

```text
Media
↓ slice 10s..20s
Video stream → Placement候補
Audio stream → Placement候補
A/Vをlink
duration = 10s
```

よ。

なら`clip()`自体もstandard extensionで構わない。

```moonbit
pub fn clip(
  media : Media,
  range : SourceSpan,
) -> ClipBundle {
  ...
}
```

そして`ClipBundle`は最終的に、

```text
TimeMap
VisualSource
AudioSource
link metadata
```

へlowerする。

つまりRust Coreは**「Clipとは何か」すら知らなくていい可能性がある**。

Coreが知るのは、

```text
このSourceをこのTimeMapで参照し、
この時間へPlacementする
```

だけ。

`Clip`は人間向けの便利概念になる。

これは結構大胆だけれど、私は好きね。

---

# 3. `trim / speed / freeze / remove`も全部TimeMap関数

これもかなり原則的。

Coreには、

```text
TimeMap
```

だけある。

例えば、

```text
local time → source time
```

の写像。

そのうえでMoonBit stdlibが、

```moonbit
trim(map, range) -> TimeMap

retime(
  map,
  range,
  rate,
) -> TimeMap

freeze(
  map,
  at,
  duration,
) -> TimeMap

remove(
  map,
  spans,
) -> TimeMap
```

を実装する。

するとVELの、

```vel
edit gameplay {
  remove 12s..18s
  speed 30s..40s = 2x
  freeze at 55s for 2s
}
```

は全部、

```text
TimeMap transformations
```

へのSugarにすぎない。

つまりRust側に、

```rust
fn freeze(...)
fn ripple_delete(...)
fn speed_change(...)
```

みたいな動画編集操作を大量に実装しなくていい。

Rust Coreは、

> **このTimeMapが妥当か**

だけチェックする。

これはかなり強い。

---

# 4. `between()`も追放

これも一見基本演算だった。

```vel
spans cuts =
  beats |> between()
```

でも、

```text
EventSet
→ SpanSet
```

なんてただの関数よ。

```moonbit
pub fn between(events : EventSet) -> SpanSet {
  ...
}
```

でいい。

するとCoreが必要なのは、

```text
Event
EventSet
Span
SpanSet
```

という**データ型だけ**。

集合演算はMoonBit。

例えば、

```text
union
intersection
subtract
expand
contract
between
nearest
```

全部追放できる。

VELにSQLもどきの集合代数を実装しなくてよくなる。

これはかなり嬉しい。

---

# 5. `horizontal / grid / fit / contain`も全部追放

空間も同じ。

Core：

```text
Region
Transform
SpatialPlacement
Composition
```

。

MoonBit：

```moonbit
horizontal(...)
vertical(...)
grid(...)
picture_in_picture(...)
cover(...)
contain(...)
center(...)
safe_area(...)
```

。

例えば、

```vel
layout horizontal {
  place left
  place right
}
```

は、

```text
std.horizontal(
  items=[left, right],
  bounds=canvas.bounds
)
```

へdesugar。

出力はただ、

```text
left  → Rect(0, 0, .5, 1)
right → Rect(.5, 0, .5, 1)
```

。

つまりCoreには`horizontal layout`という概念すらない。

**空間の事実としてRectが置かれているだけ。**

---

# 6. `crop`も微妙に追放可能

これはかなり根本的。

Coreが、

```text
Source coordinate space
Viewport
Transform
```

を持っていれば、

```vel
crop(rect(...))
```

は実際、

```text
SourceSpaceの部分領域を
新しいLocalSpaceへmappingする
```

だけ。

MoonBit stdlib：

```moonbit
pub fn crop(
  source : Visual,
  region : Region[SourceSpace],
) -> VisualPlan {
  ...
}
```

。

Core Graphには、

```text
ViewportMapping
```

だけ落とす。

こうすると、

```text
crop
pan-and-scan
reframe
cover
contain
```

が全部**座標変換を生成する方策**になる。

この整理はかなり美しい。

---

# 7. `duck()`もCoreではない

音声も同じ。

Core：

```text
Signal<T>
Curve<T>
Property<Gain>
AudioRoute
```

。

MoonBit：

```moonbit
pub fn duck(
  sidechain : Signal[Amplitude],
  base : Gain,
  amount : Gain,
  attack : Duration,
  release : Duration,
) -> Curve[Gain]
```

。

VEL：

```vel
music.gain =
  duck(
    by: narration.audio,
    base: -12dB,
    amount: -7dB,
  )
```

。

最終Core Graphでは、

```text
music.gain = concrete Curve<Gain>
```

しか残らない。

**Duckingという概念をRender backendまで持っていく必要がない。**

これも良い。

---

# 8. `fade / crossfade / transition`まで追放できる

例えばCrossfade。

Coreに、

```text
opacity curve
audio gain curve
overlapping placements
```

があれば、

```moonbit
crossfade(left, right, duration)
```

は、

```text
左のOpacity: 1 → 0
右のOpacity: 0 → 1

左のGain: ...
右のGain: ...

Placement overlap: duration
```

を生成すればいい。

つまり、

```vel
transition crossfade(12f)
```

もSurface sugar。

Coreに`Transition`型すら不要になる可能性がある。

これは判断が分かれるところだけれど、**「Transitionはprimitiveではなくmacro」**という設計は十分あり得る。

私はかなり惹かれる。

---

# 9. `scene over speech`もPolicyにできる

これも結構大きい。

```vel
scene demo over narration.demo {
  pad.end = 500ms
  ...
}
```

をCore semanticsだと考えていたけれど、

実際には、

```text
scene duration =
  narration duration
  + padding

scene内のaudio placement =
  narration at 0

次Scene =
  previous.end
```

という方策にすぎない。

なら、

```moonbit
pub fn scene_over(
  speech : Speech,
  body : SceneBody,
  padding : Padding,
) -> ScenePlan
```

でいい。

これで、

```text
speech-driven scene
music-driven scene
fixed-duration scene
content-driven scene
```

を全部Extensionとして定義できる。

VEL自体に「ナレーションが背骨」という思想を焼き付けなくて済む。

これはかなり重要ね。

---

# 10. Caption生成も当然追放

例えば、

```vel
captions from narration.words
```

も、

```moonbit
captions(
  words,
  style,
  max_chars,
  break_strategy,
) -> Array[TextPlacement]
```

。

さらに、

```text
1行何文字
句読点で切る
2行まで
強調単語だけ色変更
```

と高度化してもVELは膨らまない。

こういう**一見「標準編集機能」なものほどMoonBitへ追放する価値が高い**。

---

# さらに攻めると `Scene` もCoreでなくなる

これは思考実験として面白い。

Coreに、

```text
Named Scope
Local Time Domain
Placement[]
```

だけあれば、

`Scene`とは、

> local timeを持つ名前付きPlacement集合

に過ぎない。

つまり、

```text
Scene
```

そのものをRustの特別型にしない。

MoonBit stdlibが、

```text
Scope
+
LocalTimeline
+
Composition
```

からSceneを構築する。

同じように、

```text
Sequence
```

も、

> 複数のTemporal Scopeを並べたroot scope

でしかない。

こうすると本当にCoreは抽象的になる。

ただ、ここまで行くとGPUI側で、

```text
「これはSceneだからScene Viewで表示」
```

をどう判断するかという問題が出る。

なので私は実装上は`Scene / Sequence`くらいまではCore semantic entityとして残してもいいと思う。

**でも「Sceneの挙動」は全部MoonBit**。

この区別がいい。

---

# かなり削った場合のCore

そうするとRust側は、極論こんなところまで減る。

```text
Lattice Core
────────────────────────

Identity
  EntityId
  Reference
  Provenance

Time
  RationalTime
  TimeSpace
  Span
  TimeMap

Space
  CoordinateSpace
  Point
  Region
  Transform

Values
  Property<T>
  Curve<T>
  Signal<T>

Media
  Resource
  Stream

Graph
  VisualSource
  AudioSource
  Placement
  Composite
  AudioRoute

Structure
  Sequence
  Scene
  Scope

Collections
  EventSet<T>
  SpanSet<T>
  Collection<T>

Infrastructure
  Diagnostics
  Extension calls
  Resolution
```

そしてMoonBit stdlib側：

```text
lattice/std
────────────────────────

Temporal
  clip
  trim
  remove
  speed
  freeze
  flow
  align
  between

Spatial
  crop
  cover
  contain
  horizontal
  vertical
  grid
  pip

Audio
  fade
  crossfade
  duck
  normalize
  envelope

Editorial
  scene_over
  arrange
  montage
  replay

Text
  captions
  lower_third
  callout

etc.
```

**これくらいまで追放してもいいと思う。**

---

## 推測（示唆）

ここまで行くと、原則がかなり綺麗になる。

> **Rust Coreは「何が存在しているか」と「それが妥当か」だけ知る。  
> MoonBitは「どうやってその状態を作るか」を知る。**

例えば、

```text
freeze
```

という動詞をRustは知らない。

Rustが見るのは結果の、

```text
TimeMap
0s → source 10s
...
5.2s..6.7s → source 15.2s fixed
...
```

だけ。

`horizontal`も知らない。

```text
Placement A region = ...
Placement B region = ...
```

しか見ない。

`duck`も知らない。

```text
Gain Curve
```

しか見ない。

これ、相当気持ちいいわ。

---

ただし**一つだけ制限したい**。

MoonBit関数の結果を毎フレーム計算しながらRenderする設計にはしない。

悪い：

```text
render frame
→ MoonBit extension
→ layoutを計算
→ MoonBit extension
→ gainを計算
```

良い：

```text
Compile / Resolve
       ↓
MoonBit
       ↓
Concrete Core Graph
       ↓
Render
```

MoonBitは**編集グラフを生成するmeta layer**。

Render hot pathには入れない。

そうすれば、

- 性能予測しやすい
- lock可能
- lint可能
- IDEからGraphをinspectできる
- backend-independent

になる。

---

そしてこの設計なら、Surface VELもかなり面白くなる。

例えば、

```vel
scene demo over narration.demo {
  game[10s..20s] {
    speed = 2x
  }
}
```

という気持ちいい記法を残しつつ、

内部では実は、

```text
scene           → std.scene()
over            → std.scene_over()
game[...]       → std.clip()
speed           → std.retime()
flow            → std.flow()
```

と、**ほぼ全部MoonBit stdlibへのSugar**にできる。

VEL Compiler自体はそれらの意味をほとんど知らない。

これなら本当に「Surface syntaxは贅沢、Core semanticsは極小、知能はMoonBit」という構造になる。

私はこの方向、前よりさらに好きね。**`freeze`を追加するためにRust CoreへPRを出す必要がない世界**まで行けるのが強い。

---

# you asked

message time: 2026-08-19 08:26:57

良さそう、できるだけ言語機能に何もかも乗っかって、強めのDSLくらいにしたい

---

# chatgpt response

そうね。**独自言語を作るというより、「MoonBitを動画編集に異様に最適化したライブラリ＋薄い構文層」として使う**くらいが、今の方向には一番合ってると思うわ。

## 事実

ここまで削ると、VEL側で独自に必要なものはかなり少なくできる。

MoonBit側の普通の言語機能に、

```text
型
関数
パイプ
コレクション
クロージャ
モジュール
パターンマッチ
ジェネリクス
演算子
```

を任せてしまう。

すると例えば、わざわざVELに

```vel
each clip in montage {
  ...
}
```

という制御構文を作らなくても、

```moonbit
montage.each(fn(clip) {
  ...
})
```

で済む。

`where`も、

```moonbit
clips.filter(fn(clip) {
  clip.duration < 500.ms()
})
```

でいい。

`arrange`も、

```moonbit
shots
|> arrange(into=slots, strategy=energetic)
```

。

つまり今までVELに追加しそうだった大量の機能が、**普通の関数呼び出しになる**。

---

## 推測（示唆）

私はここからさらに一歩進めて、

> **VEL = MoonBit上のLattice DSL**

くらいまで寄せてもいいと思う。

ただし「生MoonBitを書かせる」ではなく、動画編集用の型とAPIを徹底的に整えて、見た目としてDSLっぽくする。

例えば一本のSceneが、

```moonbit
let demo =
  scene("demo", over=narration.demo, fn(ctx) {
    let gameplay =
      game["12:35".."12:43"]
      |> freeze(at=5.2.s(), duration=1.5.s())

    let damage =
      gameplay.video
      |> frame(at=5.2.s())
      |> crop(rect(0.62, 0.15, 0.22, 0.12))

    ctx.canvas(fn(c) {
      c.place(gameplay.video)
        .fill()

      c.place(damage)
        .at(5.2.s())
        .duration(1.5.s())
        .region(rect(0.55, 0.58, 0.40, 0.28))

      c.text("この数字")
        .at(5.2.s())
        .duration(1.5.s())
        .position(percent(50), percent(18))
    })

    ctx.mix(fn(m) {
      m.place(gameplay.audio)
        .gain(db(-9))

      m.place(narration.demo.audio)
    })
  })
```

くらい。

これは明らかにMoonBitだけれど、用途としてはほぼDSLよ。

---

### そうすると「Core VEL」という言語すら要らなくなる可能性がある

かなり大胆だけれど、

```text
MoonBit source
    ↓
Lattice SDK
    ↓
Project Graph
    ↓
validation
    ↓
Render Plan
```

だけで成立する。

つまり、

```text
VEL Parser
VEL AST
VEL typechecker
VEL module system
VEL formatter
VEL LSP
```

を全部消せる。

MoonBit側に乗る。

Latticeが自分で持つCompiler相当のものは、

```text
Project Graph Validator
Temporal Validator
Spatial Validator
Media Resolver
Render Planner
```

だけになる。

これは実装量としてかなり魅力的ね。

---

### ただしSurface Sugarだけは残してもいい

例えば、

```moonbit
500.ms()
2.s()
12.frames()
db(-6)
pct(50)
```

は書けるけれど、

```vel
500ms
2s
12f
-6dB
50%
```

よりは明らかに冗長。

だから私はここだけは、

> **MoonBitそのものをforkする**

とかではなく、

> **Lattice用の薄いpreprocessor**

くらいなら許してもいいと思う。

例えば、

```text
main.vel
    ↓
tiny syntax lowering
    ↓
generated .mbt
    ↓
MoonBit compiler
```

。

このpreprocessorが扱うのは本当に、

```text
500ms
12f
-6dB
50%
```

みたいな**ドメインリテラルと少数の構文Sugarだけ**。

型、関数、scope、module、collection、control flowには一切触らない。

この線なら、自作言語処理系地獄にはかなり入りにくい。

---

### さらに面白いのは、ほとんどの「動画編集機能」がライブラリになること

例えば、

```moonbit
clip(...)
trim(...)
freeze(...)
speed(...)
flow(...)
horizontal(...)
grid(...)
duck(...)
crossfade(...)
captions(...)
scene_over(...)
arrange(...)
```

全部ただの関数。

だからユーザー側でも、

```moonbit
fn dramatic_freeze(
  source : Clip,
  at : LocalTime,
) -> Composition {
  ...
}
```

と普通に定義できる。

そして、

```moonbit
demo
|> dramatic_freeze(at=5.2.s())
```

。

**これ自体がユーザー拡張になる。**

「Extensionを書く」という大げさな概念すら不要なケースが大量に出ると思う。

普通のMoonBit moduleを書けばいい。

---

### Extensionという言葉を使うのは外部能力が必要な時だけ

この整理だと、

```text
普通の編集ロジック
→ MoonBit function/module

再利用可能な編集パターン
→ MoonBit package

Beat解析・TTSなど外部処理
→ Lattice extension capability

FFmpeg/GPU/OS
→ Rust native driver
```

と分かれる。

これはかなり綺麗。

例えば`energetic_montage`なんて、本来はExtensionですらなく、

```moonbit
import "lattice/montage"

let result =
  energetic_montage(shots, beats)
```

という**普通のライブラリ**でいい。

Wasm Component境界が必要なのは、Latticeホストから特殊な権限や外部処理を受けるものだけ。

---

### 「強めのDSL」の正体

今の方向なら、私はこう定義したい。

> **VELは新しいプログラミング言語ではない。  
> MoonBitの型システムと構文を利用して構築された、動画編集用の強い内部DSLである。**

そのDSLが提供する世界は、

```text
Time
Space
Media
Scene
Placement
Composition
Signal
Property
Event
Span
```

で構成される。

ユーザーはその値を普通のMoonBitで組み合わせる。

すると、

```text
「この操作を言語に追加するべきか？」
```

という問い自体がほぼ消える。

代わりに、

```text
「これはどんな型の関数か？」
```

になる。

例えばFreezeなら、

```text
freeze : Clip × LocalTime × Duration -> Clip
```

Crossfadeなら、

```text
crossfade : Visual × Visual × Duration -> Composition
```

Duckingなら、

```text
duck : Signal<Amplitude> × DuckConfig -> Curve<Gain>
```

Flowなら、

```text
flow : Array<TemporalItem> -> Array<Placement>
```

この方がずっと扱いやすい。

---

そして私は、この方向にすると**VELという名前すら再定義できる**と思う。

```text
VEL = Video Editing Layer
```

くらいにして、

> MoonBit上に構築されたLattice標準動画編集DSL/API

という位置付けにする。

ファイル拡張子も最終的には`.mbt`でも構わないし、Lattice用preprocessorを通すなら`.vel`でもいい。

つまり今後考えるべきなのは「VELの文法」より、

**Lattice SDKの型設計と、MoonBitでどこまで動画編集コードを気持ちよく書けるか**

の方になってくるわ。

私はこの方向、かなり好きよ。独自言語を作る面白さの美味しい部分だけ残して、面倒な部分をMoonBitコンパイラへ全部押し付けられる。

---

# you asked

message time: 2026-08-19 08:29:05

じゃあ、この路線でゲーム解説実況を書いてみて

---

# chatgpt response

そうね。今回はさらに一段振り切って、**独自VEL構文をほぼ使わず、MoonBitそのもの＋Lattice SDKだけでゲーム解説動画を書く**形を試したい。

MoonBitには現在、`|>`、ラベル付き引数、optional引数、関数値、`<|`を使ったtrailing-lambda風記法があるので、こういう内部DSLを組む土台は十分ある。([MoonBit Documentation](https://docs.moonbitlang.com/en/stable/language/fundamentals.html?utm_source=chatgpt.com))

以下のLattice API自体はもちろん設計案よ。

## 推測（設計案）

題材は引き続き「Prisma Obexのフィニッシャー構成解説」。TTSでナレーションを作り、ゲーム録画からB-rollを切り、ビルド画面、Freeze、拡大、Replay、左右比較までやる。

```moonbit
/// main.mbt

fn main {
  let project =
    Project::new(
      name="prisma-obex-guide",
      video=VideoFormat::new(
        width=2560,
        height=1440,
        fps=fps(60),
      ),
      audio=AudioFormat::new(
        sample_rate=hz(48000),
      ),
    )

  //
  // Assets
  //

  let game =
    project.media(
      "game",
      "D:/Captures/Warframe/obex-test.mp4",
    )

  let build =
    project.image(
      "build",
      "./assets/obex-build.png",
    )

  let bgm =
    project.media(
      "bgm",
      "./assets/bgm.flac",
    )


  //
  // Speech
  //
  // VOICEVOX自体はLattice Coreではなくextension。
  //

  let voicevox =
    project.extension("lattice/voicevox")

  let narrator =
    voicevox.voice(
      speaker=3,
      speed=1.05,
    )

  let narration =
    Narration::new(narrator)

  let intro_voice =
    narration.say(
      """
      今回はPrisma Obexのフィニッシャー構成を見ていく。
      Incarnon込みで、実際にどこまで火力が伸びるのか確認する。
      """,
    )

  let build_voice =
    narration.say(
      """
      まずビルド。
      今回特に見たいのは、フィニッシャー倍率とクリティカル倍率の関係だ。
      """,
    )

  let demo_voice =
    narration.say(
      """
      実際に当てるとこうなる。
      この瞬間のダメージ表示に注目してほしい。
      """,
    )

  let compare_voice =
    narration.say(
      """
      通常構成と並べると差はかなり分かりやすい。
      左がフィニッシャー特化、右が汎用構成だ。
      """,
    )

  let conclusion_voice =
    narration.say(
      """
      結論として、フィニッシャーが通る相手には完全に過剰火力。
      一方で、汎用性との交換にはなる。
      """,
    )


  //
  // Scenes
  //

  let hook =
    hook_scene(game)

  let intro =
    intro_scene(
      game,
      intro_voice,
    )

  let build_explanation =
    build_scene(
      game,
      build,
      build_voice,
    )

  let demonstration =
    demo_scene(
      game,
      demo_voice,
    )

  let comparison =
    comparison_scene(
      game,
      compare_voice,
    )

  let conclusion =
    conclusion_scene(
      game,
      conclusion_voice,
    )


  //
  // Timeline
  //
  // flow()自体もstdlibのMoonBit関数。
  //

  let main =
    [
      hook,
      intro,
      build_explanation,
      demonstration,
      comparison,
      conclusion,
    ]
    |> flow()
    |> Sequence::new("main")


  //
  // Global audio
  //

  let music =
    bgm.audio
    |> loop()
    |> gain(
         duck(
           by=narration.signal(),
           base=db(-12),
           amount=db(-7),
           attack=ms(80),
           release=ms(250),
         ),
       )

  project.add_sequence(main)
  project.add_audio(music)


  //
  // Export
  //

  project.export(
    sequence="main",
    profile=ExportProfile::youtube(
      codec=VideoCodec::H265,
    ),
  )

  project.build()
}
```

ここまではかなり普通のMoonBitプログラムに見える。

で、Scene側。

---

### Hook

```moonbit
fn hook_scene(game : Media) -> Scene {
  let gameplay =
    game
    |> slice(
         from=ts("26:14"),
         to=ts("26:22"),
       )

  scene("hook", duration=s(8)) <| fn(ctx) {
    ctx.canvas() <| fn(c) {
      c.place(gameplay.video)
        |> fill()

      c.text("Prisma Obex Incarnon")
        |> at(s(2))
        |> for_(s(3))
        |> position(pct(50), pct(78))
        |> fade_in(f(8))
        |> fade_out(f(8))
    }

    ctx.mix() <| fn(m) {
      m.place(gameplay.audio)
        |> gain(db(-5))
    }
  }
}
```

ここで`fade_in()`はCore primitiveである必要すらない。

実装は例えば単に、

```moonbit
fn fade_in(
  placement : VisualPlacement,
  duration : Duration,
) -> VisualPlacement {
  placement.with_opacity(
    curve([
      keyframe(f(0), 0.0),
      keyframe(duration, 1.0, easing=EaseOut),
    ]),
  )
}
```

でいい。

つまり**普通のMoonBit関数**。

---

# Intro

音声の長さをSceneの長さにする、という方策も普通の関数にする。

```moonbit
fn intro_scene(
  game : Media,
  voice : Speech,
) -> Scene {
  let gameplay =
    game
    |> slice(
         from=ts("01:40"),
         to=ts("01:58"),
       )

  scene_over(
    name="intro",
    speech=voice,
    padding=Padding::end(ms(400)),
  ) <| fn(ctx) {
    ctx.canvas() <| fn(c) {
      c.place(gameplay.video)
        |> fill()
    }

    ctx.mix() <| fn(m) {
      m.place(gameplay.audio)
        |> gain(db(-15))

      m.place(voice.audio)
    }
  }
}
```

ここが結構大事。

独自DSLなら、

```text
scene intro over narration.intro
```

という文法を作りたくなったところを、

```moonbit
scene_over(
  name="intro",
  speech=voice,
)
```

という**ライブラリ関数一個**で済ませている。

---

# Build解説

```moonbit
fn build_scene(
  game : Media,
  build : Image,
  voice : Speech,
) -> Scene {
  let gameplay =
    game
    |> slice(
         from=ts("05:10"),
         to=ts("05:32"),
       )

  let detail =
    build
    |> crop(
         rect(
           x=pct(58),
           y=pct(18),
           width=pct(34),
           height=pct(28),
         ),
       )

  scene_over(
    name="build",
    speech=voice,
    padding=Padding::end(ms(500)),
  ) <| fn(ctx) {
    ctx.canvas() <| fn(c) {
      c.place(gameplay.video)
        |> fill()

      c.place(build)
        |> at(s(1))
        |> for_(s(6))
        |> region(
             rect(
               x=pct(8),
               y=pct(8),
               width=pct(84),
               height=pct(84),
             ),
           )
        |> contain()

      let detail_placement =
        c.place(detail)
        |> at(s(7))
        |> for_(s(5))
        |> region(
             rect(
               x=pct(52),
               y=pct(20),
               width=pct(42),
               height=pct(48),
             ),
           )
        |> contain()

      detail_placement
        |> outline(
             width=px(3),
           )

      c.text("Finisher倍率")
        |> at(s(7.5))
        |> for_(s(4))
        |> below(
             detail_placement,
             offset=px(24),
           )
    }

    ctx.mix() <| fn(m) {
      m.place(gameplay.audio)
        |> gain(db(-18))

      m.place(voice.audio)
    }
  }
}
```

ここも面白い。

`below()`すら単なる空間関数として、

```text
Placement × Placement × Offset
→ Placement
```

で実装できる。

VELに、

```text
attach
near
below
above
```

なんて文法を足す必要がない。

---

# 実演

Freeze、Crop、Callout。

```moonbit
fn demo_scene(
  game : Media,
  voice : Speech,
) -> Scene {
  let raw =
    game
    |> slice(
         from=ts("12:35"),
         to=ts("12:43"),
       )

  let gameplay =
    raw
    |> freeze(
         at=s(5.2),
         duration=s(1.5),
         audio=FreezeAudio::Silence,
       )

  let damage_frame =
    raw.video
    |> frame(at=s(5.2))

  let damage =
    damage_frame
    |> crop(
         rect(
           x=pct(62),
           y=pct(15),
           width=pct(22),
           height=pct(12),
         ),
       )

  scene_over(
    name="demonstration",
    speech=voice,
    padding=Padding::end(ms(600)),
  ) <| fn(ctx) {
    ctx.canvas() <| fn(c) {
      c.place(gameplay.video)
        |> fill()

      let zoom =
        c.place(damage)
        |> at(s(5.2))
        |> for_(s(1.5))
        |> region(
             rect(
               x=pct(55),
               y=pct(58),
               width=pct(40),
               height=pct(28),
             ),
           )
        |> contain()
        |> outline(width=px(4))

      c.text("この数字")
        |> at(s(5.2))
        |> for_(s(1.5))
        |> above(zoom, offset=px(32))

      c.arrow(
        from=point(pct(50), pct(28)),
        to=zoom.top_center(),
      )
        |> at(s(5.2))
        |> for_(s(1.5))
    }

    ctx.mix() <| fn(m) {
      m.place(gameplay.audio)
        |> gain(db(-9))

      m.place(voice.audio)
    }
  }
}
```

ここまででもほぼ全部、

```text
slice
freeze
frame
crop
contain
outline
above
```

という**関数**しか増えていない。

---

# Replayも関数化する

ゲーム解説なら多用しそうだから、ユーザー側で一度書けばいい。

```moonbit
fn replay(
  media : Media,
  from~ : SourceTime,
  to~ : SourceTime,
  rate? : Double = 0.65,
) -> Composition {
  let source =
    media
    |> slice(from~, to~)
    |> speed(rate)

  composition() <| fn(c) {
    c.place(source.video)
      |> fill()

    c.text("REPLAY")
      |> position(pct(10), pct(10))
      |> for_(s(1.5))

    c.audio(source.audio)
      |> gain(db(-9))
  }
}
```

そして、

```moonbit
let replay_part =
  replay(
    game,
    from=ts("12:34"),
    to=ts("12:43"),
  )
```

。

これ、**Extensionでも言語機能でもなく、ただのユーザー関数**よ。

私はこの感触がかなり好き。

---

# Comparison

```moonbit
fn comparison_scene(
  game : Media,
  voice : Speech,
) -> Scene {
  let left =
    game
    |> slice(
         from=ts("22:01"),
         to=ts("22:09"),
       )

  let right =
    game
    |> slice(
         from=ts("24:12"),
         to=ts("24:20"),
       )

  scene_over(
    name="comparison",
    speech=voice,
  ) <| fn(ctx) {
    ctx.canvas() <| fn(c) {
      let [left_place, right_place] =
        [left.video, right.video]
        |> horizontal(
             bounds=c.bounds(),
             gap=px(8),
           )

      c.place(left_place)
      c.place(right_place)

      c.text("Finisher")
        |> above(left_place, offset=px(24))

      c.text("General")
        |> above(right_place, offset=px(24))
    }

    ctx.mix() <| fn(m) {
      m.place(left.audio)
        |> gain(db(-18))

      m.place(right.audio)
        |> gain(db(-18))

      m.place(voice.audio)
    }
  }
}
```

`horizontal()`も、

```text
Array<Visual>
× Region
→ Array<SpatialPlacement>
```

という普通のMoonBit関数。

だからユーザーが、

```moonbit
fn golden_ratio_layout(...)
```

を書いて差し替えてもいい。

Latticeは何も知らない。

---

# Conclusion

```moonbit
fn conclusion_scene(
  game : Media,
  voice : Speech,
) -> Scene {
  let gameplay =
    game
    |> slice(
         from=ts("28:20"),
         to=ts("28:38"),
       )
    |> speed(0.85)

  scene_over(
    name="conclusion",
    speech=voice,
    padding=Padding::end(s(1)),
  ) <| fn(ctx) {
    ctx.canvas() <| fn(c) {
      c.place(gameplay.video)
        |> fill()

      c.text("フィニッシャー相手なら過剰火力")
        |> at(s(1))
        |> for_(s(5))
        |> position(pct(50), pct(80))
    }

    ctx.mix() <| fn(m) {
      m.place(gameplay.audio)
        |> gain(db(-15))

      m.place(voice.audio)
    }
  }
}
```

---

# そして普通のMoonBitを使えるのが効く

例えば「比較Sceneを複数作る」。

独自VELなら`each`だの`for`だのを設計していたところを、

```moonbit
let tests = [
  ("Corpus", ts("22:01"), ts("22:09")),
  ("Heavy Gunner", ts("24:12"), ts("24:20")),
  ("Eximus", ts("26:02"), ts("26:10")),
]

let scenes =
  tests.map(fn(test) {
    let (name, from, to) = test

    simple_demo_scene(
      name=name,
      source=game |> slice(from~, to~),
    )
  })
```

で終わる。

分岐だって、

```moonbit
fn damage_color(value : Int64) -> Color {
  if value > 1_000_000 {
    Color::Red
  } else {
    Color::White
  }
}
```

でいい。

「VELへ条件分岐を追加しよう」と考える必要そのものが消える。

---

# ユーザー定義の編集抽象化

さらにゲーム解説専用ライブラリを自分で作れる。

```moonbit
fn damage_callout(
  canvas : Canvas,
  source : Visual,
  at~ : LocalTime,
  region~ : Region[SourceSpace],
  label~ : String,
) -> Unit {
  let detail =
    source
    |> frame(at~)
    |> crop(region~)

  let placement =
    canvas.place(detail)
    |> at(at)
    |> for_(s(1.5))
    |> region(
         rect(
           x=pct(55),
           y=pct(58),
           width=pct(40),
           height=pct(28),
         ),
       )
    |> contain()
    |> outline(width=px(4))

  canvas.text(label)
    |> at(at)
    |> for_(s(1.5))
    |> above(placement, offset=px(24))
}
```

次から、

```moonbit
damage_callout(
  c,
  gameplay.video,
  at=s(5.2),
  region=damage_region,
  label="この数字",
)
```

だけ。

これが「Lattice macro」ではなく**普通のMoonBit関数**なのが大きい。

---

## この試し書きから見えるもの

私はかなりこの路線に傾くわ。

### Lattice SDK側に必要な型

```text
Project

Media
Video
Audio
Image
Speech

Sequence
Scene
Composition

Canvas
Mix

VisualPlacement
AudioPlacement

Time
Duration
SourceTime
LocalTime

Point<Space>
Region<Space>
Transform

Property<T>
Curve<T>
Signal<T>

Event<T>
EventSet<T>
Span
SpanSet

Diagnostic
```

この辺はLatticeが提供する。

### stdlib側のMoonBit関数

```text
slice
clip

flow
scene
scene_over

speed
freeze
remove
trim

frame
crop

fill
contain
cover

horizontal
vertical
grid

fade_in
fade_out
crossfade

gain
duck

outline
above
below

captions
```

全部MoonBit。

### Extension / native capability

```text
VOICEVOX
ASR
beat detection
VLM
object tracking

FFmpeg
decoder
encoder
GPU
filesystem
```

これは外界との境界。

---

そして思った以上に、**Surface VELすら要らない可能性が出てきた**。

確かに、

```text
5.2s
12f
-9dB
50%
```

をそのまま書けない不満は残る。

でも、

```moonbit
s(5.2)
f(12)
db(-9)
pct(50)
```

程度なら、私は十分手書きできる。

それと引き換えに、

- parserを作らない
- module systemを作らない
- general-purpose type checkerを作らない
- collection syntaxを作らない
- control flowを作らない
- function systemを作らない
- package managerを作らない
- 独自LSPを一から作らない

を得られるなら、交換条件としてかなり安い。

今の感触だと、**`.vel`という独自言語を作るより、`.mbt`をそのままLattice Projectのsourceにする方が本命に上がってきた**と思う。

Lattice独自なのは言語ではなく、

> **MoonBitに提供する異様に強い動画編集型システム／SDKと、その結果として生成されるProject Graph**

という方が、ずっと筋が良さそうよ。

---

# you asked

message time: 2026-08-19 08:32:35

うーん、思ったよりスッと読めなくて気持ちよくない。Rails的な思想を持ち込めないかな

---

# chatgpt response

うん。たぶん今のMoonBit版が気持ちよくなかった理由は、**動画の内容よりAPIの配管を読まされている**からよ。

`ctx.canvas()`, `c.place()`, `s(5.2)`, `pct(50)`……全部正しい。でもRailsで毎回RackやSQL ASTを書かされないのと同じで、**正しい低レベルAPIを日常の記述面に露出させる必要はない**。

## 事実

Rails的な思想から持ち込めるものは、単なるRuby風構文よりこっち。

- **Convention over Configuration**
- **80〜90%の普通のケースを異様に短くする**
- Domain Objectに自然な語彙を与える
- 暗黙の文脈をかなり利用する
- 強いdefaultを置く
- 必要なときだけ低レベルへ降りられる
- 同じ処理を何度も書かせない
- 内部では普通の汎用言語・ライブラリに落ちる

Railsの、

```ruby
has_many :comments
```

が気持ちいいのは、裏で何十個もの処理をしていないからではなく、**「この文脈ならこれだけ言えば残りは分かるだろう」という態度が強い**からよ。

VELにもそれを持ち込める。

---

# 推測（示唆）

私は方針を少し戻して、

> **MoonBitをVELそのものとして見せるのではなく、MoonBitをVELの実装言語にする**

のが良いと思う。

つまり、

```text
VEL
  Rails的な強いDomain DSL
        ↓
desugar

MoonBit / Lattice SDK
  普通のプログラム
        ↓

Project Graph
```

。

独自言語処理系を全部作るんじゃない。

**薄いSurface DSLだけ自作する。**

ここならかなり美味しいところ取りできる。

---

# 例えばゲーム解説をこう読みたい

```vel
project "obex-guide"

media game = "capture.mp4"
image build = "build.png"

voice narrator using voicevox {
  speaker = 3
  speed = 1.05
}

narration {
  intro """
    今回はPrisma Obexのフィニッシャー構成を見ていく。
    """

  build """
    まずビルド。今回重要なのはフィニッシャー倍率だ。
    """

  demo """
    実際に当てるとこうなる。
    この瞬間のダメージ表示に注目してほしい。
    """

  conclusion """
    フィニッシャーが通る相手には完全に過剰火力だ。
    """
}


sequence main {
  hook
  intro
  build
  demo
  conclusion
}
```

ここまで読んで、もう動画の大筋が分かる。

Sceneへ降りる。

```vel
scene hook {
  game[26m14s..26m22s]

  title "Prisma Obex Incarnon" {
    at 2s for 3s
    bottom 22%
    fade 8f
  }
}
```

これくらいでいい。

ゲーム素材を書いただけで、

```text
video → canvas全面
audio → mix
A/V link
duration → source range
```

を全部Conventionで補完する。

---

# `over narration`もRails的にする

```vel
scene intro over narration.intro {
  game[1m40s..1m58s]
}
```

これだけ。

Conventionとして、

```text
scene.duration = narration.duration
narration.audio → primary speech
game.video → background visual
game.audio → -15dB
BGM → automatic ducking
```

までやってしまってもいい。

もちろん明示overrideできる。

```vel
scene intro over narration.intro {
  game[1m40s..1m58s] {
    audio = -20dB
  }

  padding end 500ms
}
```

この「**普通なら書かない**」がRails的な部分ね。

---

# Build解説

前のMoonBit版はこんな配管があった。

```text
ctx.canvas
c.place
region
contain
detail_placement
outline
below
```

日常形ならもっと直接的でいい。

```vel
scene build over narration.build {
  game[5m10s..5m32s]

  show build at 1s for 6s

  crop build
    region (58%, 18%, 34%, 28%)
    as detail

  show detail at 7s for 5s {
    right 6%
    width 42%
    outline 3px
  }

  label detail "Finisher倍率" {
    below 24px
  }
}
```

これはかなり読めるでしょう。

内部では、

```text
show
crop
label
below
```

全部MoonBit stdlib関数へ落ちればいい。

---

# 実演はさらにRailsっぽくできる

```vel
scene demo over narration.demo {
  game[12m35s..12m43s] as fight

  freeze fight at 5.2s for 1.5s

  callout fight at 5.2s {
    region (62%, 15%, 22%, 12%)
    label "この数字"
    zoom 1.8x
  }

  replay fight[-1s..end] {
    speed 0.65x
    badge "REPLAY"
  }
}
```

これ、私はかなり好き。

何をしているか上から、

> 映像を置く  
> 止める  
> 注目部分を見せる  
> リプレイする

で読める。

Render Graphの構造を読む必要がない。

---

# `callout`は言語機能ではない

これが重要。

表面上は、

```vel
callout fight at 5.2s {
  ...
}
```

だけど、Compilerが特別に「Calloutとは何か」を知る必要はない。

例えばdesugarすると、

```moonbit
std::callout(
  source=fight,
  at=seconds(5.2),
  region=...,
  label="この数字",
  zoom=1.8,
)
```

でいい。

つまりRailsの、

```ruby
belongs_to :user
```

に近い。

**DSL語彙は増やしていい。Compiler semanticsは増やさない。**

ここが今までの「MoonBitへ追放」と矛盾しない。

---

# Railsなら「Context」を徹底的に利用する

例えばScene内で、

```vel
title "Hello"
```

と書いたら当然そのSceneのCanvasへ置く。

わざわざ、

```vel
canvas {
  place text(...)
}
```

とは書かない。

`audio`ならSceneのmixへ。

```vel
music bgm {
  gain -14dB
}
```

。

そして、

```vel
scene foo {
```

の中では時間リテラルもScene-localなのがdefault。

素材Rangeだけ、

```vel
game[12m35s..12m43s]
```

なのでSource-local。

つまり**文脈から明らかな座標系は書かない**。

コンパイラ内部では型を厳密に保持する。

Railsらしく、

> 人間には楽をさせるが、内部まで曖昧にはしない。

---

# 名前によるConventionも結構使える

例えば、

```vel
game[...]
```

で`game`がA/V Mediaなら両方使う。

```vel
game.video[...]
```

なら映像だけ。

```vel
game.audio[...]
```

なら音声だけ。

同様に、

```vel
show build
```

でImageならVisualとしてCanvasへ。

```vel
play bgm
```

ならAudioとしてMixへ。

動詞と型で自然に決められる。

---

# RailsのPartial / Helperっぽいものも効く

ゲーム解説で何度もやる、

```text
Freeze
Zoom
Outline
Label
Replay
```

を自分のhelperにする。

例えばMoonBit側に、

```moonbit
pub fn damage_demo(...) -> SceneFragment
```

を書いたらVELから、

```vel
use damage_demo(
  fight,
  at: 5.2s,
  region: (62%, 15%, 22%, 12%),
  label: "4.8M",
)
```

。

さらにSurface DSLへ登録すれば、

```vel
damage_demo fight at 5.2s {
  region (...)
  label "4.8M"
}
```

までSugar化できてもいい。

ただし新しいparser grammarを追加するわけではなく、

```text
IDENT expression ... block
```

という汎用DSL invocationにする。

ここはかなりRails的。

---

# つまり文法自体もRails DSL的にする

例えば基本形を、

```text
command positional* named* block?
```

くらいまで一般化する。

```vel
show build at 1s for 6s

freeze fight at 5.2s for 1.5s

label detail "Finisher倍率" {
  below 24px
}

replay fight[-1s..end] {
  speed 0.65x
}
```

これらを全部、

```text
Invocation {
  receiver?
  command
  args
  kwargs
  block
}
```

としてparseする。

**Parserは語彙を知らない。**

型checker/SDK側で、

```text
show(Image, ...)
freeze(Clip, ...)
label(VisualPlacement, String, ...)
```

を解決する。

これならDSL語彙をMoonBit package側で増やせる。

かなり強いわ。

---

# 比較シーン

```vel
scene compare over narration.compare {
  split horizontal {
    left  game[22m01s..22m09s] label "Finisher"
    right game[24m12s..24m20s] label "General"
  }
}
```

。

これも、

```text
split
left
right
label
```

はstd DSL。

低レベルに展開すると、

```moonbit
horizontal_layout(...)
```

。

でも人間にはそんなことはどうでもいい。

---

# 音声もConventionで消す

例えばナレーションがあるSceneでは、

```vel
scene demo over narration.demo {
```

と書けば、

```text
speech = 0dB
game audio = -15dB default
bgm = duck
```

。

ユーザーが変えたいときだけ、

```vel
audio {
  game -9dB
  narration -2dB
}
```

と書く。

**Mixerを毎Scene書く必要なんてなかった**のよ。

これも前のMoonBit版を読みにくくしていた大きな原因。

---

# Rails的に最初の動画を書き直すと

かなりこうなる。

```vel
project "prisma-obex-guide"

media game = "capture.mp4"
image build = "build.png"
music bgm = "bgm.flac"

voice narrator using voicevox {
  speaker 3
  speed 1.05x
}

narration {
  intro """
    今回はPrisma Obexのフィニッシャー構成を見ていく。
    """

  build """
    まずビルド。重要なのはフィニッシャー倍率だ。
    """

  demo """
    実際に当てるとこうなる。
    この瞬間のダメージ表示に注目してほしい。
    """

  compare """
    左がフィニッシャー特化、右が汎用構成だ。
    """

  conclusion """
    フィニッシャーが通る相手には完全に過剰火力だ。
    """
}

sequence main {
  hook
  intro
  build
  demo
  compare
  conclusion
}


scene hook {
  game[26m14s..26m22s]

  title "Prisma Obex Incarnon" {
    at 2s for 3s
    bottom 22%
    fade 8f
  }
}


scene intro over narration.intro {
  game[1m40s..1m58s]
}


scene build over narration.build {
  game[5m10s..5m32s]

  show build at 1s for 6s

  crop build
    region (58%, 18%, 34%, 28%)
    as detail

  show detail at 7s for 5s {
    right 6%
    width 42%
    outline 3px
  }

  label detail "Finisher倍率" {
    below 24px
  }
}


scene demo over narration.demo {
  game[12m35s..12m43s] as fight

  freeze fight at 5.2s for 1.5s

  callout fight at 5.2s {
    region (62%, 15%, 22%, 12%)
    label "この数字"
    zoom 1.8x
  }

  replay fight[-1s..end] {
    speed 0.65x
    badge "REPLAY"
  }
}


scene compare over narration.compare {
  split horizontal {
    left  game[22m01s..22m09s] label "Finisher"
    right game[24m12s..24m20s] label "General"
  }
}


scene conclusion over narration.conclusion {
  game[28m20s..28m38s] {
    speed 0.85x
  }

  title "フィニッシャー相手なら過剰火力" {
    at 1s for 5s
    bottom 20%
  }
}
```

**こっちはかなりスッと読む。**

そして重要なのは、これだけRails的にしても中身は、

```text
VEL Surface
    ↓ generic DSL parser

Invocation Tree
    ↓ resolve against MoonBit SDK

MoonBit functions
    ↓

Project Graph
```

でいいということ。

独自の`freeze`文法をCompilerへ追加しているわけじゃない。

---

## 私はこの方向ならVELを残したい

前のMoonBit直書き案では、

> 言語処理系を作らなくて済む

代わりに、

> **動画を読み書きする気持ちよさ**

をかなり売ってしまっていた。

それはLatticeにとって案外高い代償だと思う。

だから、

**MoonBitは計算モデル・型・module/package・extension機構として全面利用する。  
VELはRails routesやmigrationのような、極端にopinionatedなSurface DSLとして残す。**

この組み合わせがたぶん一番いい。

Railsから持ってくるべきなのは「Ruby」ではなく、**利用者が何をしたいか分かり切っているなら、いちいち全部言わせるな**という厚かましさね。Latticeにもそれくらいの魔法はあっていいわ。

---

# you asked

message time: 2026-08-19 08:37:15

よし、いい感じ。動画編集はMagicalでも再現可能なら困らない。ガシガシ色々決めてみよう

---

# chatgpt response

いいわ。ここからは「候補を並べる」より、**Latticeの憲法を仮決めして、その前提でSurface DSL・実行モデル・Studioの挙動まで一気に固定する**のが良さそうね。特に「Magicは許す。ただし必ず展開・固定・説明できる」を中心原則にする。

うん。ここからは**「Magicを嫌わない。隠蔽だけ嫌う」**で行くのがいいわ。

## 事実

ここまででLatticeにはかなり明確な制約ができている。

VELは人間がスッと読めるRails的なSurface DSL、計算能力と拡張はMoonBitへ追放する。Rust Coreは時間・空間・媒体・配置などの最低限の意味論だけを持つ。StudioはGPUI、Agentは外付け、履歴はGit、Resolveが必要なTTSや解析結果は固定可能にする。

この前提なら、**Magicと再現可能性はまったく対立しない**わ。

問題になるのは「暗黙に何かしたこと」ではなく、

> 同じ入力なのに何をしたのか分からない、再現できない、展開して確認できない

ことだから。

---

# 推測（設計決定案）

まずLattice v0.1の憲法をこれで仮固定したい。

1. **VELは意図を短く書くためのDSLであり、汎用言語ではない。** 制御構文、関数、ジェネリクス、集合処理などが欲しくなったらMoonBitへ行く。
2. **Magicは許可する。ただし必ず展開可能であること。** `lattice explain`で「何が暗黙に起きたか」を全部見られる。
3. **Conventionは設定を補完してよいが、編集意図を捏造してはいけない。** 音量・fit・layer・paddingなどは補える。勝手なカット、ループ、素材選択はしない。
4. **VELのDSL語彙は増やしてよい。Compiler primitiveは増やさない。** `callout`や`replay`を増やすのは安いが、Rust Coreへ`CalloutNode`を追加しない。
5. **MoonBitが動詞、Rust Coreが名詞を所有する。**
6. **Compile / Resolve / Renderは完全に分離する。**
7. **非決定的処理はResolve時にMaterializeし、必要ならlockする。**
8. **レンダリング中にはMoonBitを呼ばない。** MoonBitはGraphを生成するmeta layer。
9. **プロジェクトの正本はテキスト。** Project DBは作らない。
10. **GUIもAgentも同じProject Graphを編集する。**
11. **永続履歴はGit。Undo/RedoはStudio sessionのEditPatch。**
12. **FFmpegはbackendであって、VELの意味論ではない。**

この12個はかなり長持ちすると思う。

---

# Magicを3種類に分類する

ここは明示的に名前を付けたい。

| 種類 | 例 | 再現性 |
|---|---|---|
| **Sugar** | `title`, `freeze`, `scene over speech` | 常に決定的 |
| **Resolve** | TTS、beat解析、VLM | lockすれば決定的 |
| **Live** | 現在時刻、外部APIの最新結果 | 通常buildでは禁止 |

例えば、

```vel
scene intro over narration.intro {
  game[1m40s..1m58s]
}
```

はSugar。

展開すると、

```text
scene.duration = narration.intro.duration + convention padding
game.video → fullscreen placement
game.audio → scene game bus
narration.intro.audio → narration bus
```

になる。

一方、

```vel
narration.intro
```

をVOICEVOXから生成するのはResolve。

結果を、

```text
text hash
engine id/version
voice params
audio artifact hash
word timing
duration
```

として`lattice.lock`へ固定できる。

そして**Live依存は明示的にopt-inしない限りcommit/buildを通さない**。これはかなり強めでいい。

---

# VELは「generic invocation DSL」にする

構文は驚くほど単純にする。

```text
command args modifiers* block?
```

例えば、

```vel
show build at 1s for 6s

freeze fight at 5.2s for 1.5s

callout fight at 5.2s {
  region (62%, 15%, 22%, 12%)
  label "この数字"
}

replay fight[-1s..end] {
  speed 0.65x
}
```

Parserは`freeze`の意味を知らない。

ASTはほぼ、

```text
Invocation
  command = freeze
  args = [fight]
  modifiers:
    at = 5.2s
    for = 1.5s
```

だけ。

意味解決すると、

```text
freeze
↓
lattice/std/temporal::freeze(...)
↓
MoonBit
↓
TimeMap
```

。

これならstdlibやpackageが**新しいDSL語彙を追加できるのに、新しいgrammarを追加する必要はない**。

かなりRails的ね。

---

# 予約する構文はごく少数

私はこれくらいにしたい。

```vel
project "name"

media game "capture.mp4"

sequence main {
  intro
  demo
  outro
}

scene demo over narration.demo {
  ...
}

game[10s..20s] as fight
```

言語が本当に知るのは、

```text
declaration
block
binding/as
media range
unit literal
reference/path
invocation
```

くらい。

`at / for / over / using / as / by / from / to`あたりは汎用modifierとして予約してもいい。

---

# `=`はあまり使わない

Rails的にするなら、

```vel
speed 0.65x
gain -12dB
bottom 20%
```

でいい。

```vel
speed = 0.65x
```

にしない。

`=`が出てくるのは、本当に値をbindingしたい低レベル記述だけ。

日常のVELは**命令文というより宣言文を並べる**感じにしたい。

---

# 単位はVEL側で贅沢にする

ここは独自DSLを残した最大の恩恵なので徹底する。

```vel
5.2s
500ms
12f
-12dB
1.05x

1920px
50%
45deg

48kHz
```

MoonBitへlowerすると、

```text
seconds(5.2)
milliseconds(500)
frames(12)
decibels(-12)
ratio(1.05)
```

になる。

型checkerはMoonBit側の型を利用する。

だから人間には、

```vel
bottom 20%
```

とだけ書かせつつ、内部では、

```text
Length<CanvasNormalized>
```

くらい厳密にしていい。

---

# 時間のConventionも決める

VELでは、**現在のscopeを見れば分かる時間座標系は書かない**。

Scene内部：

```vel
at 5s
```

は`SceneTime`。

Media slice：

```vel
game[12m35s..12m43s]
```

は`SourceTime`。

Clipに対する、

```vel
fight[-1s..end]
```

は`ClipLocalTime`。

Sequence absolute timeを使いたいときだけ、

```vel
sequence.at(1m32s)
```

のように露骨にする。

見た目は魔法、内部型は厳格。これでいい。

---

# 空間も同じ

Canvas内なら、

```vel
position (50%, 80%)
region (10%, 10%, 40%, 30%)
```

はCanvas Space。

`crop`内部なら、

```vel
region (62%, 15%, 22%, 12%)
```

はSource Space。

明示したければ、

```vel
source.region(...)
canvas.region(...)
```

へ降りられる。

ここも**文脈で省略、Coreでは型区別**ね。

---

# Convention Packを正式概念にする

Rails的な厚かましさを、バラバラのimplicit ruleとして実装するより、**Convention Pack**にする。

例えば、

```vel
convention commentary
```

をProjectで選ぶ。

これが決めるのは、

```text
bare A/V media in scene
  → fullscreen video
  → linked game audio

scene over speech
  → speech durationをScene durationに
  → tail padding 400ms

game audio + narration
  → game -15dB

BGM + narration
  → default ducking

image shown without placement
  → contain + centered

text/title
  → project themeを利用
```

。

でも全部、

```bash
lattice explain scene demo
```

で見える。

別Packなら、

```vel
convention documentary
convention shorts
convention gameplay
```

も作れる。

しかもこれらは**MoonBit package**でいい。

---

# ThemeとConventionは分ける

これは重要。

```text
Convention = 編集挙動
Theme      = 見た目
```

だから、

```vel
convention commentary
theme "minimal-dark"
```

。

`minimal-dark`は、

```text
title font
caption style
outline
safe margin
default callout
colors
```

を決める。

`commentary`は、

```text
audio
timing
placement defaults
```

を決める。

これを混ぜない。

---

# SceneはCore entityとして残す

かなり削れるけれど、私は`Scene`だけは残したい。

理由はStudioが、

```text
Structure
Timeline Zoom
Preview Scope
Diagnostics Scope
Git Diff Summary
```

の単位として使えるから。

ただしSceneが**どう時間を決めるか**はCoreに持たない。

```vel
scene foo {
```

ならfixed/content-driven。

```vel
scene foo over narration.foo {
```

なら`std.scene_over()`。

```vel
scene montage over music[32s..48s] {
```

なら別Policy。

Sceneは「名前付き編集scope」でしかない。

---

# Sequenceも残す

```vel
sequence main {
  hook
  intro
  build
  demo
  compare
  conclusion
}
```

はLattice全体の読みやすさに効きすぎる。

ただしこれも、

```text
std.flow([hook, intro, ...])
```

へlowerする。

`Sequence`は意味的なroot entity、`flow`はMoonBit。

---

# bare mediaは思い切ってMagicにする

例えば、

```vel
scene intro {
  game[1m40s..1m58s]
}
```

。

これだけで、

```text
game.video → Canvas fill
game.audio → Mix
Scene.duration → media duration
```

でいい。

もし映像だけなら、

```vel
show game.video[...]
```

。

音だけなら、

```vel
play game.audio[...]
```

。

普通のケースを一行にする方が価値が高い。

---

# Magicの限界も決める

私はこのルールがいい。

> **Magicは「足りない設定」を補完してよい。  
> Magicは「足りない素材や編集判断」を補完してはいけない。**

例えばSceneが12秒なのに背景動画が8秒しかない。

ここで勝手に、

```text
loop
freeze last frame
slow down
```

はしない。

Compiler ErrorまたはWarning。

```text
demo background ends 3.2s before scene end

Quick fixes:
  Hold last frame
  Loop
  Extend source range
  Shorten scene
```

人間かAgentに選ばせる。

この境界はかなり大事。

---

# Stable identityも方針を決める

VELは何でもIDを書くようにはしない。

```vel
title "Hello"
```

は匿名でいい。

でも後から、

```vel
label title ...
```

のように参照したくなったら、

```vel
title "Hello" as intro_title
```

と名前を付ける。

Studioで匿名要素に直接編集を掛けた結果、永続参照が必要になったら、IDEが自動で、

```diff
-title "Hello"
+title "Hello" as intro_title
```

と名前を付ける。

**名前は必要になった瞬間に生える**。

Rails的でいいわ。

---

# MoonBitの役割も整理し直す

MoonBitには三段階の使い方がある。

| 用途 | 例 |
|---|---|
| **stdlib implementation** | `freeze`, `flow`, `callout`, `duck` |
| **user helper/package** | `damage_demo`, `warframe_build_card` |
| **capability extension** | TTS、ASR、beat detector |

単なる編集抽象化にWasm Component ABIは不要。

普通のMoonBit moduleでいい。

外界への能力が必要なものだけExtension Hostを通す。

これは大幅に簡単になる。

---

# Compilation pipelineも固定したい

```text
main.vel
   ↓
VEL parser
   ↓
Invocation AST
   ↓
Convention expansion
   ↓
MoonBit lowering
   ↓
generated MoonBit
   ↓
MoonBit typecheck / execute
   ↓
Semantic Project Graph
   ↓
Lattice validation
   ↓
Resolve
   ↓
Resolved Project + lattice.lock
   ↓
Render Plan
   ↓
Backend
```

そして必須で**source map**を持つ。

MoonBit側で型エラーが出ても、

```text
generated.mbt:183
```

ではなく、

```text
scenes/demo.vel:14:3

freeze fight at 5.2s ...
^^^^^^
```

へ戻す。

ここは絶対条件。

---

# Lattice Compilerが自分でやることはかなり減る

独自に実装するのは、

```text
VEL parse
contextual desugar
source mapping

Project Graph validation

temporal/spatial/media lint
```

くらい。

汎用的な、

```text
function
generic
module
control flow
type inference
collection
package
```

はMoonBit。

この分業ならかなり納得できる。

---

# `lattice explain`を一級機能にする

Magicが多いほど重要。

例えば、

```bash
lattice explain scene intro
```

で、

```text
scene intro

Convention:
  commentary@1.0

Duration:
  narration.intro.duration
  = 8.412s
  + tail padding 400ms
  = 8.812s

Visual:
  game.video[1:40..1:58]
  → crop: none
  → fit: cover
  → placement: canvas

Audio:
  game.audio
  → gain -15dB

  narration.intro.audio
  → gain 0dB

Music:
  master BGM
  → ducked by narration
```

。

さらに、

```bash
lattice explain --code scene intro
```

なら**完全展開した低レベルVEL/MoonBit相当**を見せる。

Magicへの恐怖がかなり消える。

---

# Studioにも「Explain Magic」を置く

Canvasで要素を選ぶとInspectorに、

```text
Position
  50%, 80%

Source
  convention: commentary/title

Explicit?   no

[Make Explicit]
```

。

`Make Explicit`すると、

```diff
 title "Hello"
+{
+  position (50%, 80%)
+}
```

みたいにVELへ書き戻す。

逆に、

```text
[Use Convention]
```

で冗長な指定を消す。

これはめちゃくちゃLatticeらしい。

---

# Git diffもMagicを理解する

例えばConvention Packを変えただけで大量の映像結果が変わる。

普通のGit diffだと、

```diff
-convention commentary@1
+convention commentary@2
```

だけ。

Latticeでは、

```text
Semantic Diff

34 scenes affected

Audio:
  default game gain
  -15dB → -17dB

Scene padding:
  400ms → 300ms

Caption safe margin:
  8% → 10%
```

までCompilerが出す。

Magicを使うなら**semantic diffが重要になる**。

---

# TTSはかなり厚く統合する

これは今まで通り。

```vel
voice narrator using voicevox {
  speaker 3
  speed 1.05x
}

narration {
  demo """
    実際に当てるとこうなる。
    この瞬間のダメージ表示に注目してほしい。
  """
}
```

Speech resultは、

```text
audio
duration
sentences
words
phonemes
marks
```

を持つ。

なので将来的には、

```vel
callout fight when narration.demo says "この瞬間" {
  ...
}
```

みたいなSugarも作れる。

重要なのは、この`when ... says`もCore primitiveにしないこと。

MoonBit側で、

```text
Speech.words → Event
```

に落とす。

---

# v0.1のstdlib語彙も決めてしまう

最初から何百個もいらない。

**Temporal**

```text
trim
speed
freeze
remove
replay
flow
align
```

**Visual**

```text
show
crop
fill
cover
contain
zoom
outline
mask
```

**Composition**

```text
split
grid
overlay
callout
title
label
badge
```

**Audio**

```text
play
gain
fade
duck
mute
```

**Speech**

```text
voice
narration
caption
```

これくらいあればゲーム解説はかなり書ける。

全部MoonBit stdlib実装。

---

# これでゲーム解説はこの程度まで縮められる

```vel
project "prisma-obex-guide"

convention commentary
theme "minimal-dark"

media game "capture.mp4"
image build "build.png"
music bgm "bgm.flac"

voice narrator using voicevox {
  speaker 3
  speed 1.05x
}

narration {
  intro """
    今回はPrisma Obexのフィニッシャー構成を見ていく。
  """

  build """
    まずビルド。重要なのはフィニッシャー倍率だ。
  """

  demo """
    実際に当てるとこうなる。
    この瞬間のダメージ表示に注目してほしい。
  """

  compare """
    左がフィニッシャー特化、右が汎用構成だ。
  """

  conclusion """
    フィニッシャーが通る相手には完全に過剰火力だ。
  """
}

sequence main {
  hook
  intro
  build
  demo
  compare
  conclusion
}

scene hook {
  game[26m14s..26m22s]

  title "Prisma Obex Incarnon" {
    at 2s for 3s
    bottom 22%
    fade 8f
  }
}

scene intro over narration.intro {
  game[1m40s..1m58s]
}

scene build over narration.build {
  game[5m10s..5m32s]

  show build at 1s for 6s

  crop build
    region (58%, 18%, 34%, 28%)
    as detail

  show detail at 7s for 5s {
    right 6%
    width 42%
    outline 3px
  }

  label detail "Finisher倍率" {
    below 24px
  }
}

scene demo over narration.demo {
  game[12m35s..12m43s] as fight

  freeze fight at 5.2s for 1.5s

  callout fight at 5.2s {
    region (62%, 15%, 22%, 12%)
    label "この数字"
    zoom 1.8x
  }

  replay fight[-1s..end] {
    speed 0.65x
    badge "REPLAY"
  }
}

scene compare over narration.compare {
  split horizontal {
    left  game[22m01s..22m09s] label "Finisher"
    right game[24m12s..24m20s] label "General"
  }
}

scene conclusion over narration.conclusion {
  game[28m20s..28m38s] {
    speed 0.85x
  }

  title "フィニッシャー相手なら過剰火力" {
    at 1s for 5s
    bottom 20%
  }
}
```

このくらいなら、ようやく**「コードを読んでいる」というより「編集指示を読んでいる」**ところまで来ていると思う。

そして裏側はかなり堅い。

```text
Magic Surface
     ↓
MoonBit functions
     ↓
Strict typed graph
     ↓
locked resolution
     ↓
deterministic render
```

この組み合わせなら、私はむしろ**Magicをかなり強くしていい**と思う。

Latticeの個性は「何も暗黙にしない」ではなく、**「大量に暗黙にするくせに、いつでも全部説明できる」**にした方が面白いわ。

---

# you asked

message time: 2026-08-19 08:39:49

簡易なコンパイラを書いて、動かしてみて。言語は何でもいい

---

# chatgpt response

やるわ。今回は「構文の美しさ」より、**今のRails的VELが本当に parse → Magic展開 → Core IR まで落ちるか**を見る最小プロトタイプにする。`scene / media slice / title / freeze / callout / replay / convention`あたりまで実装して、実際にサンプルをコンパイルしてみる。

動いたわ。**「Rails的Surface DSL → Magic展開 → CoreっぽいIR → lint」**まで、最低限だけど一本通せた。

## 事実

今回の試作はPythonで書いた。対応したSurfaceはこの辺。

- `project / convention / theme`
- `media / image / music`
- `narration`
- `sequence`
- `scene ... over narration.x`
- `game[12m35s..12m43s] as fight`
- `title`
- `freeze`
- `callout`
- `replay`
- `speed`

例えば、

```vel
scene demo over narration.demo {
  game[12m35s..12m43s] as fight

  freeze fight at 5.2s for 1.5s

  callout fight at 5.2s {
    region (62%, 15%, 22%, 12%)
    label "この数字"
    zoom 1.8x
  }

  replay fight[-1s..end] {
    speed 0.65x
    badge "REPLAY"
  }
}
```

が、実際にこういうIRへ落ちた。

```text
scene policy
  scene-over-speech
  speech = narration.demo
  convention = commentary

source fight
  asset = game
  source range = 755s..763s

freeze
  target = fight
  at = 5.2s
  duration = 1.5s
  audio = silence

callout
  ↓
  frame(source, at)
  crop(frame, region)
  place(crop)
  outline(place)
  label(place)

replay
  source = fight
  range = -1s..end
  speed = 0.65x

convention expansion
  fight.video → canvas-fill
  fight.audio → -15dB
  narration.demo → speech track
```

つまり、前に言っていた

> **`callout`はCompiler primitiveじゃなく、複数Core操作へ展開されるstdlib語彙**

という構造が、少なくとも簡易版ではちゃんと成立したわ。

さらに`commentary` conventionもMagicとして動いている。

```text
scene X over narration.Y
```

から自動的に、

```text
Scene duration
Speech placement
Game audio -15dB
Bare A/V → fullscreen
```

が生成された。

TTSは本物を叩かず、今回は文字数からdurationを返す**決定論的mock resolver**にしている。

---

lintも実際に効いた。

最終実行では、

```text
WARNING LAT-TIME-103:
callout-composition extends 0.617s past scene boundary

WARNING LAT-TIME-103:
text extends 1.417s past scene boundary
```

が出た。

つまり、

```vel
callout fight at 5.2s {
  ...
}
```

という一見問題なさそうな記述が、TTSで決まったScene長に対してはみ出していることを、レンダリング前に検出できた。

これはかなりLatticeらしい。

途中、日本語文字列のdecodeを雑に書いて`"この数字"`を文字化けさせるバグも出たけど、そこも修正して再実行済みよ。

## 推測（示唆）

今回実際に書いて動かしてみて、**generic invocation DSL路線はかなり現実味がある**と思った。

特に良かったのは、parserそのものは`callout`の意味をほとんど知らなくても成立したところ。

今はprototypeなので分岐を直書きしているけれど、次の構造へそのまま発展させられる。

```text
VEL parser
   ↓
Invocation AST

"callout"
   ↓ symbol resolution

MoonBit stdlib
   ↓

Core IR fragments
```

つまり将来的にはPythonコードにある、

```python
if head == "callout":
    ...
```

を消して、

```text
resolve DSL word "callout"
↓
MoonBit package lattice/std/callout
```

へ持っていける。

それと、今回かなり重要だったのが、**Magic展開後のIRに`origin`を残したこと**。

例えば、

```json
{
  "gain_db": -15,
  "origin": "convention:commentary"
}
```

となっている。

これがあれば将来、

```text
Why is this -15dB?

→ commentary convention
→ default game audio under narration
```

とStudioで説明できる。

`lattice explain`の土台としてかなり自然よ。

次にやるなら私は、このPython版を少しだけ発展させて、

```text
1. Invocation Registry
2. Convention expansion registry
3. Source map
4. explain command
5. IR validator
```

まで作る。

**そこまで行けば、「VELのParserは固定、語彙とMagicは外から増える」というLatticeの核心を、かなり小さいコードで実証できる**と思う。

ファイルも残してあるわ。

[簡易VELコンパイラ](sandbox:/mnt/data/lattice_compiler_demo.py)  
[今回コンパイルしたVEL](sandbox:/mnt/data/demo.vel)  
[生成されたIR](sandbox:/mnt/data/demo.ir.json)

---

# you asked

message time: 2026-08-19 08:52:53

Moonbitにベタっとくっつくよりwasm取り入れるのがいいかも、という着想を得た。どうかしら

---

# chatgpt response

かなりいい着想だと思うわ。**MoonBitをLatticeの第二言語として固定するのではなく、Wasmを「拡張の実行境界」にして、MoonBitはその上に乗る有力な実装言語の一つにする**方が、設計として一段強い。

うん。**私はその方が一段いい設計になると思う。**  
MoonBitを「Latticeの第二言語」にするより、**Wasm ComponentをLatticeの拡張境界にして、MoonBitは推奨実装言語の一つにする**方が綺麗だわ。

## 事実

WebAssembly Component Modelはまさに、実装言語から独立したコンポーネント間インターフェースを作るための仕組みで、WITが型付きのinterface/worldを定義する。`world`では「そのComponentが何をexportし、何をhostからimportするか」まで契約として表現できる。([Bytecode Alliance Component Model](https://component-model.bytecodealliance.org/design/wit.html?utm_source=chatgpt.com))

MoonBitは現在、このComponent Modelを公式にサポートしていて、WITから`wit-bindgen`でMoonBit bindingを生成してComponentを実装できる。つまりMoonBitを採用しても、**Lattice ↔ MoonBit固有FFI**を設計する必要はない。([MoonBit Documentation](https://docs.moonbitlang.com/ja/latest/toolchain/wasm/component-model-tutorial.html?utm_source=chatgpt.com))

WITには単なるrecord/listだけでなく`resource`もあり、実体を直接コピーせずhandleとしてComponentへ渡すモデルを取れる。これは巨大Mediaを扱うLatticeにかなり重要よ。([Bytecode Alliance Component Model](https://component-model.bytecodealliance.org/design/wit.html?utm_source=chatgpt.com))

Rust hostならWasmtimeがComponent Modelを実行でき、memoryのresource limiterやCPU実行量のfuel/epoch interruptionも提供している。特にfuelは決定論的な中断に使える。([Wasmtime](https://docs.wasmtime.dev/api/wasmtime/struct.Config.html?utm_source=chatgpt.com))

---

## 推測（示唆）

私はスタックをこう変えたい。

```text
                         Lattice

                    VEL Surface DSL
                           │
                           ↓
                    Invocation AST
                           │
                    command resolution
                           │
             ┌─────────────┴─────────────┐
             ↓                           ↓
     Wasm Components               Rust Core
   conventions / stdlib         semantic primitives
   helpers / analyzers                │
             │                         │
             └──────────┬──────────────┘
                        ↓
                   Core Project IR
                        ↓
                     validate
                        ↓
                      resolve
                        ↓
                    Render Plan
                        ↓
               Native Rust Backend
                FFmpeg / GPU / I/O
```

そして、

```text
MoonBit ─┐
Rust    ─┼─→ Wasm Component ─→ Lattice
Go/etc. ─┘
```

という位置付けにする。

**LatticeはMoonBitを知らない。Latticeが知っているのはWITだけ。**

これはかなり強い。

---

# MoonBitは「公式推奨SDK」に降格させる

例えば、

```text
lattice-sdk-moonbit
```

を公式に提供する。

だから通常は、

```moonbit
pub fn freeze(...) -> CoreFragment {
  ...
}
```

を書けば簡単にExtensionを作れる。

でも原理的にはRustで書いてもいい。

Latticeの仕様書には、

```text
MoonBit API
```

ではなく、

```text
lattice:dsl/lower@0.1.0
lattice:analysis/events@0.1.0
lattice:resolve/speech@0.1.0
```

という**WIT contract**が載る。

この方が長期的に圧倒的に自由ね。

---

# もっと面白いのはstdlibまでWasmにできること

例えば、

```vel
freeze fight at 5.2s for 1.5s
```

をParserはただ、

```text
Invocation {
  command: "freeze"
  args: [...]
}
```

にする。

Registry：

```text
freeze
    ↓
lattice/std-temporal@1.3.0
    ↓
Wasm Component
```

。

Componentが、

```text
Input:
  source = fight
  at = 5.2s
  duration = 1.5s

Output:
  TimeMap mutation
  Audio placement mutation
```

を返す。

Rust Coreからすると、

> Freeze？　知らん。  
> 妥当なTimeMapが来たので受け取る。

でいい。

同様に、

```text
flow
callout
replay
horizontal
grid
duck
crossfade
scene-over-speech
caption
```

までWasm側へ追放できる。

**VELの語彙の大半が、versioned Wasm packageになる。**

これはかなり好き。

---

# Convention Packとの相性が異常にいい

さっき話していたRails的Magic、

```vel
convention commentary
```

も、

```text
commentary
↓
lattice/convention-commentary.wasm
```

にしてしまえる。

例えば、

```text
bare A/V media
→ fullscreen video
→ audio -15dB

scene over narration
→ speech-driven duration
→ padding 400ms

music under speech
→ ducking
```

を全部このComponentが展開する。

そしてlockには、

```text
convention:
  component: lattice/convention-commentary
  version: 1.4.2
  digest: sha256:...
```

を残す。

これなら、

> **Magicの意味そのものまでversion固定できる。**

すごくいい。

---

# `lattice explain`もさらに強くなる

例えば、

```bash
lattice explain demo
```

で、

```text
scene demo

Expanded by:
  lattice/convention-commentary@1.4.2
  sha256:ab12...

Invocation:
  scene-over(narration.demo)

Generated:
  scene duration     6.083s
  narration gain     0dB
  game gain         -15dB
  canvas placement   cover
```

。

`freeze`なら、

```text
Expanded by:
  lattice/std-temporal@2.1.0

freeze fight at 5.2s for 1.5s

Generated:
  TimeMap segment:
    5.2s..6.7s → source 5.2s

  audio:
    silence 5.2s..6.7s
```

。

Magicが**どのComponentのどのversionから来たか**まで説明できる。

これは再現可能性との相性がものすごくいい。

---

# WITは用途別に分けたい

万能な、

```text
plugin.do_anything(ProjectContext)
```

は作りたくない。

例えば、

```text
lattice:dsl
    lowering

lattice:convention
    project/scene expansion

lattice:layout
    spatial arrangement

lattice:arrangement
    temporal arrangement

lattice:lint
    diagnostics

lattice:analysis
    media → observations

lattice:resolve
    external → locked artifact
```

くらいにworld/interfaceを分ける。

すると、

```text
stdlib freeze component

imports:
  core types only

exports:
  lowering
```

。

VOICEVOXなら、

```text
imports:
  speech synthesis capability
  cache/artifact storage

exports:
  speech resolver
```

。

Beat detectorなら、

```text
imports:
  media reader

exports:
  event analyzer
```

という具合。

WIT worldはimport/exportを契約として記述できるので、このcapability分割と非常に相性がいい。([Bytecode Alliance Component Model](https://component-model.bytecodealliance.org/design/worlds.html?utm_source=chatgpt.com))

---

# 「権限」も自然に設計できる

例えば`freeze.wasm`には、

```text
network    ×
filesystem ×
clock      ×
random     ×

Core types ✓
```

しか渡さない。

つまり悪意以前に、**ネットワークへアクセスする方法そのものをimportしない**。

一方TTS resolverには、

```text
network ✓
artifact store ✓
```

など必要な能力だけHostが提供する。

「plugin manifestに`network=false`と書いたから信用する」のではなく、

> **WIT worldに能力が存在しなければ呼べない**

という設計にできる。

ただしWasmを使えば勝手に完全sandboxになる、というほど雑ではなく、host側でmemory/CPU制限や公開するimportsをきちんと設計する必要はある。Wasmtime側にはそのためのresource/fuel制御がある。([Wasmtime](https://docs.wasmtime.dev/examples-interrupting-wasm.html?utm_source=chatgpt.com))

---

# Mediaは絶対にWasmへ丸ごと渡さない

ここは重要。

悪い設計：

```text
4K frame
↓ copy
Wasm linear memory
↓ process
copy
↓
Rust
```

を60fpsでやる。

そんなものは自分で橋を架けてから燃やしているようなものよ。

代わりにWITの`resource`を利用して、

```wit
resource media {
    duration: func() -> duration;
    video-info: func() -> video-info;
}

resource audio-reader {
    read: func(range: span) -> list<f32>;
}
```

みたいに、**host-owned resourceへのhandle**を渡す。WITのresourceはまさに、コンポーネント間で実装本体を共有せずhandleとして扱うための型よ。([Bytecode Alliance Component Model](https://component-model.bytecodealliance.org/design/wit.html?utm_source=chatgpt.com))

ただ、それでも大量のpixel処理自体をWasm stdlibへやらせる必要はない。

例えば、

```text
crop()
```

Wasm側が行うのは、

```text
CropPlan {
  source
  region
}
```

を生成すること。

**実際のpixel cropはRust/GPU backend。**

この、

```text
Wasm = planning
Rust = execution
```

は維持したい。

---

# すると三層がものすごく綺麗になる

```text
VEL
────────────────────────
「こう編集したい」

Wasm Components
────────────────────────
「それは具体的にはこういうGraphになる」

Rust Core / Renderer
────────────────────────
「そのGraphを検証して実行する」
```

例えば、

```vel
callout fight at 5.2s {
  region (...)
  label "この数字"
}
```

↓

Wasm：

```text
frame
crop
place
outline
text
```

↓

Rust：

```text
validated Core IR
↓
Render Plan
```

。

この分離、今まで考えた中でかなり完成度が高い。

---

# さらに「ユーザーhelper」もWasmへcompileできる

ユーザーがMoonBitで、

```moonbit
pub fn warframe_damage_callout(...) {
  ...
}
```

を書く。

↓

```text
moon build
↓
warframe-tools.wasm
```

。

VELでは、

```vel
use "warframe-tools"

damage_callout fight at 5.2s {
  ...
}
```

。

つまり、

```text
VEL package ecosystem
      =
Wasm Component ecosystem
```

になってくる。

MoonBit package managerそのものへLatticeの運命を預ける必要もなくなる。

---

# Pure ComponentとResolver Componentを分ける

ここもかなり重要。

### Pure

```text
freeze
flow
layout
callout
captions-from-known-words
```

```text
same input
+ same component digest
→ same Core IR
```

を要求する。

network/clock/randomは与えない。

さらに強く再現性を要求したければ、Wasmtimeにはmemory/resource制限や、決定論的に実行量を制限できるfuelがある。([Wasmtime](https://docs.wasmtime.dev/examples-interrupting-wasm.html?utm_source=chatgpt.com))

### Resolver

```text
TTS
ASR
VLM
beat analysis depending on implementation
```

は、

```text
input
↓
Component
↓
resolved artifact
↓
lattice.lock
```

。

以降はresolve結果を読む。

これで今まで話してきた、

```text
✓ deterministic
◇ resolved
⚠ live
```

の分類がWasm Componentの性質そのものになる。

---

# 逆にWasm化しないもの

ここも決めやすい。

```text
VEL parser
Project IR
Project validator
dependency graph
cache
GPUI
FFmpeg
decoder
encoder
GPU compositor
filesystem
Git integration
```

はRust。

特にProject IRは**Latticeの憲法**だから、Extension側へ持っていかない。

Componentが返すものは最終的にLattice Core型へlowerされる。

---

## 私なら今スタックをこう更新する

```text
                    Lattice Studio
                      Rust / GPUI
                           │
                    Lattice CLI
                           │
                     VEL Compiler
                        Rust
                           │
                    Invocation AST
                           │
                  ┌────────┴─────────┐
                  │  Wasmtime Host   │
                  └────────┬─────────┘
                           │ WIT
       ┌───────────────────┼───────────────────┐
       │                   │                   │
 Official stdlib      Convention Packs    User Packages
 Wasm Components      Wasm Components     Wasm Components
       │                   │                   │
   MoonBit etc.        MoonBit etc.        any language
       └───────────────────┼───────────────────┘
                           ↓
                    Lattice Core IR
                         Rust
                           │
                 Validate / Resolve
                           │
                     Render Plan
                           │
                  Native Rust Drivers
                           │
                FFmpeg / GPU / Media
```

これなら**MoonBitは重要だけど交換可能**になる。

私は前の、

> Rustは機械、VELは記述、MoonBitは賢いこと

から少し変えて、

> **RustはLatticeそのもの、VELは人間との境界、Wasmは拡張との境界、MoonBitはWasmを書く一番気持ちいい候補**

くらいが今は一番しっくり来るわ。

特にLatticeみたいに**「Magicを大量に許すが、それを固定・説明・再現したい」システムでは、Wasm Componentのversion付き・型付き・capability付き境界がMoonBit直接統合よりずっと本質に合っている**と思う。

---

# you asked

message time: 2026-08-19 09:26:43

仕様を守るにあたって、形式手法（前Quintを挙げた）とPBTあたりをやりたいね

---

# chatgpt response

かなり相性いいわ。Latticeは「自由なMagic」を許す代わりに、**Core IRと変換規則だけは絶対に壊れない**ことが価値になるから、形式手法とPBTを後付け品質保証ではなく設計の一部に置く意味がある。特にQuintは「状態遷移」、PBTは「値と変換」の検証に分担させるのが良さそう。

かなりやりたい。しかもQuintとPBTを**同じことに二重で使うんじゃなく、Quintは「状態機械と許される遷移」、PBTは「値変換と代数則」**に割り振ると、Latticeの設計にものすごく合うわ。

うん。**Latticeは形式手法を入れる理由がかなり明確な部類**だと思う。Magic・Wasm拡張・lock・incremental compileを全部許すなら、「実装を慎重に書く」より**許される状態遷移を別に定義して実装を殴る**方が安心できるわ。

## 事実

Quintは状態機械と不変条件を記述でき、シミュレータで実行列を生成し、モデルチェッカーではモデル上の到達可能状態を検査できる。現在はApalacheとTLCを`quint verify`から利用できる。シミュレータはモデル検査そのものではないが、ランダムな実行列から不変条件違反を探せる。([Quint](https://quint-lang.org/docs/what-does-quint-do?utm_source=chatgpt.com))

さらに現在のQuintにはRust向けの**Quint Connect**があり、Quint仕様をoracleとしてモデルベーステストを行う経路が公式に用意されている。モデルから大量のtraceを生成し、実装側で同じactionを実行して状態を比較する、という使い方になる。([Quint](https://quint-lang.org/docs/model-based-testing?utm_source=chatgpt.com))

Rust側のPBTなら`proptest`が現在1.11.0で、Hypothesis系の生成・shrinkingを提供している。([Docs.rs](https://docs.rs/crate/proptest/latest?utm_source=chatgpt.com))

---

# 推測（設計）

私は検証を三層にする。

```text
                Quint
        ─────────────────
        状態機械・不変条件
              ↓ traces
          Quint Connect
              ↓
┌──────────────────────────────┐
│        Rust implementation   │
│                              │
│ proptest ─ 値・変換・代数則 │
│ unit tests ─ 具体的回帰      │
└──────────────────────────────┘
```

重要なのは、

> **Quintで全部証明しようとしない**

ことね。

動画のcrop計算やTimeMapの細かい算術をQuintで書き始めたら辛い。逆に、

> Editした後に古いResolve結果でRenderできてしまうか？

みたいな問題をproptestだけで完全に殴ろうとすると、状態遷移の組み合わせがつらい。

---

# Quintで最初に書くもの

私はまず**Lattice Build Protocol**を一個だけ形式化する。

状態はこんな程度。

```text
Dirty
  ↓ compile
Compiled
  ↓
NeedsResolve
  ↓ resolve
Resolved
  ↓ plan
Planned
  ↓ render
Rendered
```

ただし途中で、

```text
Edit
Component upgrade
Convention change
Lock change
Media replacement
Cache eviction
```

が割り込む。

ここが重要。

例えば、

```text
Resolved
   │
   │ component updated
   ↓
NeedsResolve
```

へ戻らなければいけない。

でも実装バグで、

```text
Resolved
   │ component updated
   ↓
Resolved  ← stale artifact
```

になったら再現性が死ぬ。

こういうのはQuintが得意な形よ。

---

# 最初に置きたい不変条件

かなり強く決めてしまっていい。

### 1. Stale Render禁止

```text
Renderable
⇒
compiled_source_hash == current_source_hash
```

さらに、

```text
render_plan.project_revision
==
current_project_revision
```

。

編集後に古いRender Planを実行できない。

---

### 2. Resolve artifactのidentity

Resolve結果は必ず、

```text
input hash
component digest
parameters
capability version
```

に対応している。

概念的には、

```text
artifact.valid
⇒
artifact.key ==
hash(input, component, params, abi)
```

。

VOICEVOXで声を生成したあとExtensionをupgradeしたのに、古い音声を「新しいResolve結果」として扱ってはいけない。

---

### 3. Pure componentは状態を持たない

```text
same component digest
+
same canonical input
=
same canonical output
```

を**Latticeの契約**にする。

これは数学的にWasm全般が勝手に保証してくれるという話ではなく、Latticeの`pure` component contractとして要求する。

Pure Componentにはclock/network/random/filesystemをimportさせない設計とも対応する。

---

### 4. ResolveなしRender禁止

必要なInvocationが、

```text
resolved
```

でも、

```text
pure
```

でもない場合、

```text
renderable == false
```

。

Magicが勝手に実行時Resolveされるのは禁止。

---

### 5. Lockは意図を変更しない

`lattice.lock`更新は、

```text
VEL semantic intent
```

を変更してはならない。

変わるのは、

```text
Unresolved Invocation
→ Materialized Artifact
```

だけ。

---

### 6. Derived Graphには必ず由来がある

Core IRのderived nodeについて、

```text
user source
OR
stdlib component
OR
convention component
OR
resolved artifact
```

のどれから生まれたか追跡可能。

つまり、

```text
DerivedNode
⇒ provenance != ∅
```

。

`lattice explain`の形式的な裏付けになる。

---

# Quintが特に見つけてほしいバグ

例えば、これをモデルチェッカーに総当たりさせたい。

```text
compile
resolve
plugin upgrade
render

compile
resolve
edit
undo
render

compile
resolve A
resolve B
upgrade A
render

compile
cache hit
media replace
render

compile
resolve
lock update
checkout old VEL
render
```

人間のテストだと「まあここはinvalidateされるだろ」で抜けるところよ。

Quintなら有限化したモデルについて、操作順の組み合わせそのものを探索できる。違反時にはcounterexample traceを返すので、その実行列をそのまま回帰テストへ持っていける。([Quint](https://quint-lang.org/docs/checking-properties?utm_source=chatgpt.com))

---

# PBTは別方向から猛烈に殴る

Rust側では私は`proptest`をかなり広く使いたい。

## TimeMap

ここはPBTの宝庫。

例えば、

```text
identity(map) == map
```

```text
reverse(reverse(map)) == map
```

```text
split(map, t)
→ concat(left, right) == map
```

```text
trim(full_range) == identity
```

```text
speed(map, 1x) == map
```

さらに有理数時間にしておけば、

```text
speed(2x)
→ speed(0.5x)
```

のような変換も丸め誤差なしでかなり強く検査できる。

---

# Parser / Surface DSL

これも徹底的にPBT。

```text
parse(format(ast)) == ast
```

```text
format(parse(format(parse(src))))
==
format(parse(src))
```

つまりformatter idempotence。

さらに重要なのが、

```text
desugar(short)
==
desugar(expanded)
```

。

例えば、

```vel
scene intro over narration.intro {
  game[10s..20s]
}
```

と完全明示版から**同じCore IRが生成される**ことをPBTで確認する。

Rails的Magicを増やすなら、これが相当重要になるわ。

---

# Convention Pack

これもpropertyが書ける。

例えば`commentary`なら、

```text
Scene has Speech
⇒ speech is routed to narration bus
```

```text
bare A/V + speech
⇒ game audio default <= narration gain
```

とか。

もっと本質的には、

```text
expand(convention, project)
```

を2回掛けても、

```text
expand(expand(x)) == expand(x)
```

つまり**idempotent**であってほしい場合が多い。

Convention展開が二重適用されて音量が-15dB→-30dBになったら面白くないからね。

---

# EditPatch

Undo/Redoにも非常に効く。

可逆Patchなら、

```text
apply(inverse(p), apply(p, graph))
==
graph
```

。

互いに独立したPatchなら、

```text
apply(a, apply(b, g))
==
apply(b, apply(a, g))
```

も期待できる。

もちろん全部のPatchが可換ではないから、

```text
disjoint(a, b)
⇒ commute(a, b)
```

というpropertyにする。

---

# Spatial

Transformにも代数則がかなりある。

```text
identity ∘ T == T
```

```text
T ∘ inverse(T) == identity
```

invertibleなもの限定で。

Regionなら、

```text
crop(full_canvas) == identity
```

```text
contain(result, bounds)
⇒ result ⊆ bounds
```

など。

---

# Wasm境界

ここもPBTをかなり使える。

例えばPure componentについて、

```text
run(component, input)
==
run(component, input)
```

を大量の生成入力で確認。

加えて、

```text
serialize(input)
→ Wasm
→ deserialize(output)
```

の境界をfuzz/PBTする。

WITは言語非依存の型付きinterfaceを定義するので、このboundary testをMoonBit実装、Rust実装など複数言語へ共通化しやすい。([Bytecode Alliance Component Model](https://component-model.bytecodealliance.org/design/wit.html?utm_source=chatgpt.com))

---

# そして一番やりたいのが Quint × PBT

ここがかなり強い。

例えばQuintでは抽象的に、

```text
Edit
Compile
Resolve
UpgradeComponent
Plan
Render
```

というActionだけ定義する。

Quint Connectで、

```text
Edit
Compile
Resolve A
Resolve B
Upgrade A
Compile
Render
```

みたいなtraceを生成。

Rust側Driverは、

```rust
match action {
    Edit => ...,
    Compile => ...,
    Resolve(id) => ...,
    ...
}
```

で実装を本当に動かす。Quint公式のMBTもこの「仕様からaction列と期待状態を作り、実装側で再生する」構造になっている。([Quint](https://quint-lang.org/docs/model-based-testing?utm_source=chatgpt.com))

そして**各Actionの具体的payloadをproptestで生成する**。

```text
Edit
  scene id      ← proptest
  duration      ← proptest
  source hash   ← proptest

Resolve
  component     ← proptest
  params        ← proptest

Upgrade
  old/new digest ← proptest
```

。

要するに、

```text
Quint
  「どんな順番で起きるか」

proptest
  「具体的にどんな値で起きるか」
```

と分担する。

これは相当強いと思う。

---

# 実装状態をSpecへ射影する関数

これを一本、非常に重要なAPIとして作りたい。

```rust
fn abstract_state(workspace: &Workspace) -> SpecState
```

例えば現実のLattice内部には、

```text
AST
IR
cache objects
file handles
GPUI state
Wasm stores
```

が大量にある。

Quintはそんなものを知らなくていい。

射影すると、

```text
source_revision
compiled_revision

required_resolves
resolved_invocations

component_digests

lock_entries

plan_revision

renderable
```

くらいだけ。

Quint traceを一手実行するたび、

```text
actual Rust state
↓ abstract_state()
SpecState
```

として、Quintの期待状態と比較する。

**この射影関数が仕様と実装の橋**になる。

---

# Specの配置もこうしたい

```text
spec/
├─ build_protocol.qnt
├─ resolution.qnt
├─ component_lifecycle.qnt
├─ edit_session.qnt
└─ cache_invalidation.qnt

crates/
├─ lattice-core/
├─ lattice-compiler/
└─ lattice-wasm/

tests/
├─ model_based/
│  ├─ build_protocol.rs
│  └─ resolution.rs
│
└─ properties/
   ├─ time_map.rs
   ├─ spatial.rs
   ├─ parser.rs
   ├─ desugar.rs
   ├─ convention.rs
   ├─ edit_patch.rs
   └─ component_abi.rs
```

これくらい分かれていると美しい。

---

## 私なら最初に検証する順番

いきなり動画編集全体を形式仕様化しない。

**第一弾はこの三つだけ。**

```text
Quint
1. Compile / Resolve / Plan / Render lifecycle
2. component version + lattice.lock invalidation

proptest
3. TimeMap algebra
4. Surface VEL ↔ Core IR desugaring
5. EditPatch inverse
```

特にQuintの第一号は**Build Lifecycle**がいいと思う。

今のLatticeで一番「絶対に仕様を守ってほしい」のはCropの見た目じゃなく、

> **古いMagic結果、古いComponent、古いCacheを間違って「最新」としてRenderしない**

ことだから。

しかもQuintにはRust向けのMBT経路まで今はあるので、仕様を書いて終わりではなく、**その仕様のtraceをそのまま実装テストへ食わせられる**。([Quint](https://quint-lang.org/docs/model-based-testing?utm_source=chatgpt.com))

これはLatticeみたいな「魔法は強いが、中身は機械的でありたい」プロジェクトにはかなり理想的な使い方だと思うわ。

---

# you asked

message time: 2026-08-19 09:29:11

goldenをおくべきところも考える

---

# chatgpt response

うん。Latticeだとgoldenはかなり効くわ。特に**「Magicをどう展開したか」「人間にどう説明するか」みたいな、数学的propertyでは捕まえにくい契約**を固定するのに向いている。

## 事実

役割を分けるなら、私はこう考える。

```text
Quint
  状態遷移が正しいか

PBT
  値変換・代数則が正しいか

Golden
  具体的な入力が、期待した意味・表示へ落ちるか
```

だからgoldenを置くべきなのは、主に**人間が意味を確認できる境界**ね。

---

## 推測（設計案）

### 1. Surface VEL → 展開結果

ここは最重要golden。

例えば、

```vel
scene demo over narration.demo {
  game[12m35s..12m43s] as fight

  freeze fight at 5.2s for 1.5s

  callout fight at 5.2s {
    region (62%, 15%, 22%, 12%)
    label "この数字"
  }
}
```

に対して、

```text
golden/compile/commentary-demo/
├─ input.vel
├─ expanded.vel
└─ semantic.json
```

を置く。

`expanded.vel`は、

```text
scene duration = narration.demo.duration + 400ms
game.video -> canvas fill
game.audio -> -15dB
narration.demo -> narration bus

freeze:
  fight[5.2s..6.7s] -> hold frame
  audio -> silence

callout:
  frame(fight, 5.2s)
  crop(...)
  place(...)
  outline(...)
  label(...)
```

くらい人間に読める形式。

**Rails的Magicの契約そのもの**になるわ。

---

### 2. Convention Pack

これはgoldenを大量に置いていい。

例えば、

```text
golden/conventions/commentary/
├─ bare-media/
├─ scene-over-speech/
├─ image-default/
├─ music-ducking/
└─ explicit-override/
```

`commentary@1.2`を変更して、

```text
game gain -15dB → -17dB
```

になったらgoldenが一斉に落ちる。

そこで、

> 「これは仕様変更だからgolden更新」

と明示できる。

ConventionはMagicの塊だから、PBTだけでは「気持ちいい挙動」を守れない。ここはgoldenが本職よ。

---

### 3. `lattice explain`

**絶対golden。**

例えば、

```bash
lattice explain scene demo
```

の出力をそのまま保存する。

```text
golden/explain/
├─ scene-over-speech.txt
├─ callout.txt
├─ freeze.txt
├─ convention-override.txt
└─ resolved-speech.txt
```

LatticeではExplainabilityを機能として約束したいから、

```text
Magicの結果は正しいが、
なぜそうなったか表示されなくなった
```

も普通にregressionなのよ。

さらに`origin`やComponent versionも検査できる。

---

### 4. Diagnostics

これもかなりgolden向き。

```vel
scene demo over narration.demo {
  game[0s..4s]
}
```

に対して、

```text
warning LAT-TIME-101

scene "demo" lasts 8.4s
but background "game" covers only 4.0s

uncovered:
  4.0s..8.4s

possible fixes:
  - hold last frame
  - extend source
  - shorten scene
```

をgolden。

特に、

- diagnostic code
- source span
- entity
- temporal range
- spatial range
- Quick Fix

をまとめて見る。

PBTでは「必ずエラーが出る」は検査できても、**ちゃんと役に立つエラーなのか**は見づらいからね。

---

### 5. Semantic Diff

これもLattice固有でかなり重要。

入力を、

```text
before.vel
after.vel
```

にして、

```text
expected.diff.txt
```

をgoldenにする。

例えば、

```diff
-convention commentary@1
+convention commentary@2
```

から、

```text
34 scenes affected

Audio defaults:
  game gain: -15dB → -17dB

Scene padding:
  400ms → 300ms
```

まで出せることを固定する。

Git連携を売りにするなら、ここはかなり価値が高い。

---

### 6. Core IRは「選んで」golden

これは注意したい。

Rawな内部IR全部を毎回goldenにすると、

```text
Entity IDの実装変更
fieldの並び替え
内部最適化
```

だけで大量に壊れる。

なので、

```text
Project Graphそのもの
```

ではなく、

```text
Normalized Semantic IR
```

を別途出すのがいい。

例えば、

```json
{
  "scene": "demo",
  "duration": "8.4s",
  "placements": [
    {
      "source": "fight.video",
      "range": "0s..8.4s",
      "layout": "fill"
    }
  ]
}
```

くらい。

つまり、

> **実装表現をgoldenにせず、仕様上意味のある射影をgoldenにする。**

これはかなり重要よ。

---

### 7. Render Plan

これも同じ。

Coreから、

```text
Decode
Crop
TimeMap
Composite
Mix
```

へどうlowerされたかはgoldenにする価値がある。

ただし、

```text
FFmpeg command lineそのもの
```

をLattice全体のgoldenにはしない。

Backend adapterのtestsでは、

```text
Render Plan
→ expected FFmpeg filtergraph
```

をgoldenにしてもいいけど、上位層とは分ける。

```text
golden/render-plan/
golden/backends/ffmpeg/
```

ね。

---

### 8. Wasm Component lowering

これもかなり良い。

例えば、

```text
freeze.wasm
+
Invocation
```

から、

```text
expected CoreFragment
```

をgolden。

```text
golden/components/std-temporal/
├─ freeze-basic/
├─ freeze-with-audio/
├─ replay/
└─ speed/
```

。

そしてMoonBitで実装してもRustで実装しても、**同じgolden suiteに通せる**。

これはWasmを言語非依存境界にしたメリットがかなり出る。

---

### 9. Lockfile / Resolve

`lattice.lock`は人間がGit管理する予定ならgoldenを置いていい。

例えばTTS mock resolverで、

```vel
narration {
  intro "Hello"
}
```

↓

```toml
[[resolve]]
kind = "speech"
input_hash = "..."
component = "mock-tts"
component_digest = "..."
artifact = "..."
duration = ...
```

。

ただしhash自体が実装詳細なら、テスト用に固定hash関数を使うか、

```text
<ANY_HASH>
```

的にnormalizeした方がいい。

---

# Visual goldenも欲しい。ただし少量

例えば64×64や320×180の固定入力で、

```text
crop
contain
cover
mask
opacity
composite
freeze
simple text
```

を実際にレンダリング。

```text
golden/render/images/
├─ crop.png
├─ split-horizontal.png
├─ mask-circle.png
├─ opacity.png
└─ callout.png
```

。

ただしここはpixel-perfectにしすぎると、

```text
OS
GPU
font rasterizer
FFmpeg version
```

で落ちる。

なので、

- Core compositor primitivesはpixel exactを狙う
- GPU/backend跨ぎはperceptual tolerance
- textは固定test font/environment、あるいは画像比較を弱める

くらい。

**visual goldenを2000枚作るのはやめた方がいい。** メンテ地獄になる。

---

# Audio goldenも似ている

WAVそのものを大量に置くより、

```text
input PCM
↓
gain / fade / crossfade / duck
↓
expected PCM
```

を極小サンプルで固定。

例えば32 sampleだけ。

あるいは、

```text
sample values
peak
RMS
duration
```

のnormalized JSONをgoldenにする。

Duckingみたいに複雑なら、PBTとgoldenを両方使えばいい。

---

# Formatterにもgolden

これは普通に、

```text
ugly.vel
↓ lattice fmt
pretty.vel
```

。

ただし、

```text
formatter idempotence
```

自体はPBT。

つまり、

```text
具体的な美しいレイアウト
→ golden

何回formatしても変わらない
→ PBT
```

という分担ね。

---

# Goldenにしない方がいいところ

ここもかなり大事。

### TimeMapの正しさ

```text
reverse(reverse(x)) == x
split + concat == original
```

はPBT。

具体的なJSONを100件goldenにしても弱い。

### Build lifecycle

```text
edit → resolve → component upgrade → render
```

でstale artifactを使わない、とかはQuint。

### Cache

cache file layoutをgolden化しない。

cacheは捨てられる実装詳細。

### GPUI widget tree

これもgoldenにしない。

必要ならStudioの重要画面だけscreenshot test。

### Raw AST

Parserの実装を変更しづらくなるだけなら避ける。

Surface syntax → normalized representationの境界だけ見る方がいい。

---

# ディレクトリ構造

私なら最初からかなり露骨に分ける。

```text
tests/
├─ golden/
│  ├─ surface/
│  │  ├─ parse/
│  │  └─ format/
│  │
│  ├─ desugar/
│  ├─ conventions/
│  ├─ explain/
│  ├─ diagnostics/
│  ├─ semantic-ir/
│  ├─ semantic-diff/
│  ├─ lockfile/
│  │
│  ├─ components/
│  │  ├─ std-temporal/
│  │  ├─ std-spatial/
│  │  └─ commentary/
│  │
│  ├─ render-plan/
│  └─ render/
│     ├─ image/
│     └─ audio/
│
├─ properties/
│  ├─ time_map.rs
│  ├─ parser.rs
│  ├─ desugar.rs
│  ├─ spatial.rs
│  └─ edit_patch.rs
│
└─ model/
   ├─ build_protocol.rs
   └─ resolution.rs

spec/
├─ build_protocol.qnt
└─ resolution.qnt
```

かなり役割が分かる。

---

## 一番大事な原則

goldenについて一つ決めるなら、

> **Goldenは「内部実装」を凍結するためではなく、「人間が認識する仕様」を凍結するために使う。**

だと思う。

Latticeなら特に、

```text
VELを書いた
↓
どんなMagicが起きた？
↓
どんな意味になった？
↓
何がおかしければ何と怒られる？
↓
Git上では何が変わったように見える？
```

この辺が全部golden向き。

逆に、

```text
TimeMapの全組み合わせが正しい？
状態遷移が常に安全？
```

はQuint/PBTへ任せる。

要するに、**形式手法で「絶対に壊れてはいけないこと」を守り、goldenで「Latticeらしい振る舞い」を守る**。この三本立てはかなり強いと思うわ。

---

# you asked

message time: 2026-08-19 09:49:47

OTIOのこと、考えておいたほうがいいかも

---

# chatgpt response

そうね。OTIOは**競合というより、Latticeの内部モデルを殴るための既存の共通語彙**としてかなり見ておく価値があるわ。特に `Timeline / Track / Clip / Gap / Transition / RationalTime / TimeRange / MediaReference` の切り方と、何をあえて表現しないかが参考になる。

そうね。**OTIOは今の段階で一度ちゃんと意識しておいた方がいい**。ただし私は、Lattice CoreをOTIO準拠にするより、

> **OTIOを「編集タイムラインの共通交換境界」として第一級対応する**

くらいが一番いいと思うわ。

## 事実

OpenTimelineIOはそもそも、**editorial cut informationのAPI＋交換形式**として設計されている。媒体そのものを格納するコンテナではなく、カットの順序・長さ・外部Mediaへの参照などを持つ。CoreはC++のin-memory data modelを持ち、時間処理は依存のない`opentime`として分かれている。([GitHub](https://github.com/AcademySoftwareFoundation/OpenTimelineIO/blob/main/README.md?utm_source=chatgpt.com))

基本語彙もLatticeとかなり重なる。

```text
Timeline
Stack
Track
Clip
Gap
Transition
Marker
MediaReference

RationalTime
TimeRange
```

`Clip`はMediaReferenceとsource rangeを持ち、Track上にはClipだけでなくGapやTransitionなどを配置できる。時間は`RationalTime(value, rate)`と`TimeRange(start_time, duration)`で表現される。([OpenTimelineIO](https://opentimelineio.readthedocs.io/en/latest/api/python/opentimelineio.schema.html?utm_source=chatgpt.com))

またOTIOは交換ハブとしてかなり明確な思想を持っていて、nativeの`.otio` JSONだけでなく、AAF、CMX 3600 EDL、FCP XMLなどへのadapter群がある。現在はnative adapter以外の多くが別のplugin package/repositoryへ分離されている。([GitHub](https://github.com/AcademySoftwareFoundation/OpenTimelineIO/blob/main/docs/tutorials/adapters.md?utm_source=chatgpt.com))

一方で、OTIOは**レンダリング意味論そのものを全面的に規定する形式ではない**。例えばClipにEffectを付けられても、そのEffectが再生結果へどう影響するかはapplication-specificとされている。Transitionも表現できるが、再生側が描画できなければ無視してもTimeline全体の長さには影響しない、という設計になっている。([OpenTimelineIO](https://opentimelineio.readthedocs.io/en/v0.16.0/tutorials/otio-timeline-structure.html?utm_source=chatgpt.com))

---

## 推測（示唆）

この最後の点がかなり重要。

私は、

```text
VEL / Wasm Magic
        ↓
Lattice Semantic IR
        ↓
OTIO
```

とはしたくない。

正確には、

```text
                  Lattice Semantic IR
                   /              \
                  /                \
        Render Plan              OTIO View
            ↓                       ↓
       FFmpeg/GPU              NLE interchange
```

がいい。

**OTIOをLattice IRそのものにはしない。**

理由は、LatticeにはOTIOよりかなり豊かな意味が生えつつあるから。

```text
Scene
Convention provenance
Speech / TTS
Resolve artifacts
Wasm invocation
Canvas
Region / CoordinateSpace
Composition
Signal / Property / Curve
Magic expansion
Semantic diagnostics
```

この辺までOTIOへ押し込もうとすると、`metadata`や独自Schemaを大量に使うことになって、結局「OTIOの皮を被ったLattice IR」になる。

それなら素直にLattice IRを持った方がいいわ。

---

# 逆に、OTIOから盗むべきものはかなりある

特に**時間モデル**。

今Latticeで、

```text
SourceTime
SceneTime
SequenceTime
RationalTime
Span
```

を設計しているけれど、OTIOの`RationalTime / TimeRange`は必ず比較対象にしたい。

少なくとも、

```text
TimeRange = start + duration
```

を基本にして、range endpointの扱いを厳密にするところや、media側のavailable rangeとtimeline上で使用しているsource rangeを区別するところは参考になる。([OpenTimelineIO](https://opentimelineio.readthedocs.io/en/latest/tutorials/time-ranges.html?utm_source=chatgpt.com))

つまりCore設計時に、

> **この時間演算はOTIO/OpenTimeと相互変換可能か？**

をpropertyとして置いてもいいくらい。

---

# `Track / Gap`についてもいい比較対象になる

我々は途中から、

> TrackをSurface VELへ露出しすぎない

方向へ行った。

これはそのままでいい。

ただ内部ではOTIOとのinteropのため、

```text
Lattice Sequence/Scene/Placement
       ↓ projection
OTIO Stack/Track/Clip/Gap
```

を作れるようにしておく価値がある。

例えば、

```vel
sequence main {
  intro
  demo
  outro
}
```

はLatticeとしてはScene構造だけれど、OTIO export時には、

```text
Timeline
└ Stack
  ├ Video Track
  │ ├ intro clips...
  │ ├ demo clips...
  │ └ outro clips...
  └ Audio Track
    ...
```

へflattenできる。

つまり**Latticeの人間向け構造と、OTIOのeditorial構造を分離したまま接続する**。

---

# Importもかなり魅力的

例えばPremiere/Resolve/FCPなどからOTIOを得られるなら、

```bash
lattice import project.otio
```

で、

```text
OTIO Timeline
 ↓
Lattice Core Placements
 ↓
generated VEL
```

まで持っていける。

ただしgenerated VELは最初からRails的に綺麗にはならない。

例えば、

```vel
sequence imported {
  ...
}
```

の中身はかなり低レベルになるでしょう。

そこから、

```text
Refactor → Extract scenes
Refactor → Recognize flow
Refactor → Collapse to convention
```

で**Latticeらしい高レベル記述へ持ち上げる**。

これ、Studioの機能として結構面白い。

---

# Exportはさらに分かりやすい

```bash
lattice export otio
```

でNLEへ渡す。

ただし当然Lossyになり得る。

例えば、

```vel
callout fight at 5.2s { ... }
```

を、

```text
Lattice
↓ Magic expansion
crop / overlay / text
↓
OTIO
```

にしても、相手NLEで完全再現できる保証はない。

だからExport時に、

```text
OTIO compatibility report

✓ cuts
✓ timing
✓ source references
✓ markers

~ transitions approximated
~ retime partially represented

✗ Lattice callout composition
✗ dynamic signal modulation
✗ TTS provenance
```

くらい出したい。

OTIO自体がeditorial interchange中心でEffectの具体的な描画意味を規定していない以上、この「何がlossyかを説明する」設計は必要になる。([OpenTimelineIO](https://opentimelineio.readthedocs.io/en/v0.16.0/tutorials/otio-timeline-structure.html?utm_source=chatgpt.com))

---

# OTIO Adapter自体をWasm側に追い出せる

ここも今のArchitectureと合う。

```text
OTIO JSON
   ↓
lattice/adapter-otio.wasm
   ↓
Core Graph fragment
```

逆も、

```text
Core Graph projection
   ↓
adapter-otio.wasm
   ↓
.otio
```

。

つまりRust Coreに、

```text
AAF
FCPXML
OTIO
EDL
```

を全部実装しない。

まずOTIOだけ公式adapterとして用意して、

```text
Lattice ↔ OTIO ↔ 他の編集環境
```

を狙う。

OTIO自身にもAAF/FCP XML/EDLなどへのadapter ecosystemがあるので、この経路はかなり筋がいい。([GitHub](https://github.com/AcademySoftwareFoundation/OpenTimelineIO/blob/main/docs/tutorials/adapters.md?utm_source=chatgpt.com))

ただしLatticeから外部NLEまでの変換品質は各adapterが表現できる範囲に依存するので、「OTIOを通せば全部完全互換」とは扱わない方がいい。

---

# テストにもOTIOを使える

これが個人的にはかなり気になる。

例えばCore Time semanticsについて、

```text
Lattice IR
↓ export OTIO
↓ import OTIO
↓
Lattice IR'
```

で、

```text
editorial_projection(IR')
==
editorial_projection(IR)
```

をPBTできる。

もちろんCanvasやTTSなどOTIOにないものは射影から除く。

つまり、

```text
π_otio(import(export(x))) = π_otio(x)
```

というproperty。

これはInteropのテストとしてかなり強い。

Goldenも、

```text
tests/golden/otio/
├ basic-cut/
├ gap/
├ nested-track/
├ transition/
├ markers/
└ retime/
```

みたいに置ける。

---

# そしてOTIOの「割り切り」も参考になる

私はここが意外と大事だと思う。

OTIOは、

> **編集判断の交換をする。動画編集アプリの全機能を共通化しようとはしない。**

という境界がはっきりしている。([GitHub](https://github.com/AcademySoftwareFoundation/OpenTimelineIO/blob/main/README.md?utm_source=chatgpt.com))

Latticeも、

```text
Core IRですべてのNLE機能を抽象化する
```

という野心には行かない方がいい。

Lattice CoreはLattice自身が意味を保証すべきPrimitiveだけ。

他の世界との交換は、

```text
Projection
Adapter
Compatibility report
```

でやる。

これはWasm Extensionを採用した考え方とも完全に同じね。

---

なので私は今、OTIOをこう位置付けたい。

> **OpenTimelineIO = Latticeの外向きEditorial ABI。**

Wasmが、

```text
Lattice ↔ Extension
```

のABIなら、

OTIOは、

```text
Lattice ↔ Editing ecosystem
```

のABI。

内部モデルは独立させる。でも**Lattice IRを設計するときは「OTIOへ綺麗に射影できる編集概念か」を常に横目で見る**。

これくらいの距離感が一番得だと思うわ。

---

# you asked

message time: 2026-08-19 11:55:36

図形、装飾について考えてみる

---

# chatgpt response

そうね。ここはかなり重要だわ。図形・装飾は「動画編集の脇役」に見えるけど、**ゲーム解説や実況ではテキストと同じくらい頻繁に使う**。しかも、設計を雑にすると `shape(type="rect", ...)` の巨大な汎用オブジェクトになって死ぬ。

## 事実

まず分けた方がいいのは、**幾何そのもの**と**意味を持った装飾**ね。

Lattice Coreが知るべきなのは、たぶんこの程度。

```text
Geometry
  Point
  Rect
  RoundedRect
  Ellipse
  Path
  Line

Paint
  Fill
  Stroke
  Gradient
  Opacity

Transform
  Position
  Scale
  Rotation

Spatial relation
  Region
  Anchor
  CoordinateSpace
```

一方、

```text
矢印
吹き出し
囲み
ハイライト
下三分の一
字幕背景
比較ラベル
ダメージ数字用Callout
ゲームHUD風パネル
```

みたいなのは、**Core primitiveではなくWasm/stdlib側で組み立てる**のがいいと思う。

つまり、

```text
Core:
Path + Stroke + Fill + Transform

stdlib:
arrow()
callout()
badge()
highlight()
lower_third()
```

ね。

---

## 推測（示唆）

私は図形の入口をかなりRails的にしたい。

例えば単純な矩形なら、

```vel
rect {
  region (10%, 10%, 40%, 20%)
  fill "#0008"
  radius 12px
}
```

円なら、

```vel
circle {
  center (50%, 50%)
  radius 80px
  stroke 4px
}
```

線なら、

```vel
line {
  from (20%, 30%)
  to (70%, 60%)
  width 3px
}
```

ただし、実際の日常編集ではもっと意味的なSugarを使う。

```vel
outline damage {
  width 4px
  radius 8px
}
```

とか、

```vel
highlight build_detail {
  fill "#ff04"
  pulse 1.2x
}
```

とか。

`outline`はCore primitiveじゃなくて、最終的には

```text
target bounds
→ rounded rect
→ transparent fill
→ stroke
→ placement
```

へ展開する。

---

### 「対象に付く装飾」を強くしたい

ゲーム解説だと、

```vel
label detail "Finisher倍率" {
  below 24px
}
```

みたいな記述が気持ちいい。

ここで`detail`の位置が動いたらラベルも追従する。

同様に、

```vel
outline detail
arrow from caption to detail
badge fight "REPLAY"
```

と、装飾は絶対座標より**対象との関係**で書ける方がいい。

なので内部的には、

```text
Decoration
  target: EntityRef
  relation: Above | Below | Around | PointTo | Attach
```

みたいな概念をCoreへ入れたくなるけど、私はここでも踏みとどまりたい。

Coreは単に、

```text
SpatialConstraint / AnchorRef
```

くらいまで。

`below()`や`around()`という意味付けはWasm側。

---

### 矢印は意外と深い

例えば、

```vel
arrow from caption to damage
```

と書いたとき、欲しいのは単なる直線じゃない。

対象のBounding Boxを見て、

```text
caption右端
↓
damageの左上寄り
```

みたいに自動でendpointを選んでほしい。

しかも、

```text
画面外に出ない
文字に重ならない
短すぎない
```

みたいなlayout policyもある。

なので矢印は、

```text
Path primitive
```

ではなく、

> **Pathを生成するlayout function**

と考えた方がいい。

Wasm側で、

```text
arrow(source_rect, target_rect, policy)
→ Path
```

。

Lattice Coreには最終Pathしか残らない。

---

### 装飾にもThemeを効かせる

これはかなり重要。

例えば、

```vel
callout fight at 5.2s {
  region (...)
  label "この数字"
}
```

で、毎回、

```text
stroke 4px
radius 8px
label background ...
font ...
arrow width ...
```

なんて書きたくない。

Themeが、

```text
callout.stroke
callout.radius
callout.label
callout.arrow
highlight.fill
badge.style
```

を持てばいい。

だからSurfaceでは、

```vel
callout ...
```

だけ。

必要なら、

```vel
callout ... {
  style "warning"
}
```

とか、

```vel
callout ... {
  stroke 6px
}
```

でoverride。

Rails的Magicがかなり効く場所ね。

---

### 図形のAnimationは特別扱いしない

ここも今までの`Property<T>`設計が効く。

```vel
rect {
  scale {
    0f 0.8x
    6f 1.05x ease-out
    10f 1.0x
  }
}
```

とか、

```vel
highlight damage {
  opacity {
    0f 0
    4f 1
    end 0
  }
}
```

でもいい。

つまり、

```text
position
size
radius
stroke width
fill opacity
path points
```

全部Animation可能なProperty。

「図形Animation」という別機能を作らなくて済む。

---

### Pathは必要だけど、手書きは低レベル

たぶん最終的には自由形状も必要。

```vel
path {
  move (10%, 20%)
  line (40%, 20%)
  curve ...
  close
}
```

でもこれはSurfaceのescape hatch。

普通は、

```vel
arrow
outline
bracket
underline
callout
```

を使う。

SVG的なPathまでVELに露出はするけど、日常語彙にはしない。

---

### MaskとShapeは同じGeometryを共有する

ここはかなり綺麗にできる。

```vel
circle as face_region {
  center (50%, 50%)
  radius 20%
}
```

を、

```vel
mask webcam with face_region
```

にも、

```vel
outline face_region
```

にも使える。

つまりGeometryは、

```text
描画するとShape
alphaへ使うとMask
cropへ使うとRegion
hit-testへ使うとSelection
```

に再利用できる。

これはCoreでGeometryをちゃんと一級化する強い理由になるわ。

---

### 「装飾」はCompositionとして作る

例えばCalloutは、

```text
target crop
+
outline
+
arrow
+
label
+
optional background dim
```

の小さなComposition。

Lower Thirdも、

```text
rect background
+
title
+
subtitle
+
entry animation
```

。

つまりLatticeでは、装飾を専用Nodeとして増やすより、

> **小さなCompositionを生成するWasm関数**

として扱うのが一番筋がいい。

そうするとユーザーも、

```text
lattice/std-decoration
```

だけでなく、自分で、

```text
warframe/damage-callout
warframe/mod-card
youtube/chapter-title
```

みたいな装飾パッケージを作れる。

---

### ゲーム解説なら、このくらいの語彙が最初に欲しい

v0.1なら、図形プリミティブとしては`rect / circle / line / path / text`くらいで十分。

その上のstdlib装飾は、

```text
outline
highlight
arrow
label
badge
callout
divider
dim
```

くらいあればかなり戦える。

`lower_third`や`speech_bubble`みたいなものは後でもいい。

---

例えばゲーム解説Sceneなら、かなりこう書ける。

```vel
scene demo over narration.demo {
  game[12m35s..12m43s] as fight

  freeze fight at 5.2s for 1.5s

  region damage on fight
    (62%, 15%, 22%, 12%)

  highlight damage {
    at 5.2s for 1.5s
  }

  label damage "4.8M" {
    below 24px
  }

  arrow from label to damage

  dim fight {
    except damage
    opacity 35%
  }
}
```

これはかなり読みやすい。

そして内部では、

```text
region
→ geometry

highlight
→ rect + fill/stroke

label
→ text + relative layout

arrow
→ generated path

dim
→ overlay + mask
```

に全部lowerできる。

私はここもやっぱり、**CoreにはGeometryとPaintとSpatial relationだけ置いて、装飾の意味は全部Wasmへ逃がす**のが一番いいと思うわ。図形はCore、装飾は魔法。かなり綺麗。

---

# you asked

message time: 2026-08-19 13:15:17

GUIからコーディングエージェントへ編集地点情報を指差しできるようにしておきたいな

---

# chatgpt response

そうね。これは**かなり早い段階でデータモデルに入れておきたい**。Lattice Studioから外付けCoding Agentへ「これ」「この瞬間」「この領域」を渡せると、チャット欄に説明を書く必要がほぼなくなるわ。

## 事実

GUIで人間が「指しているもの」は、単なるEntity IDでは足りない。

例えばPreview上でダメージ数字をクリックした場合、人間の意図には少なくとも、

```text
Project
Scene
Playhead
選択Entity
画面上のRegion
Source Media上の位置
VEL source location
現在のRevision
```

が混ざっている。

だから私はこれを一級の **`EditPointer`** として持ちたい。

```rust
struct EditPointer {
    workspace: WorkspaceId,
    revision: Revision,

    scene: Option<EntityId>,
    selection: Vec<EntityId>,

    playhead: Option<SceneTime>,
    temporal_range: Option<TimeRange<SceneSpace>>,
    spatial_region: Option<Region<CanvasSpace>>,

    source_refs: Vec<SourceReference>,
    source_locations: Vec<SourceLocation>,
}
```

ただしこれは永続Project Graphではなく、**Studio session上のコンテキスト値**ね。

---

## 推測（示唆）

私はUXとして、かなり露骨に **「Point to Agent」** を作りたい。

例えばCanvasで対象を選択して、

```text
右クリック

Copy Lattice Pointer
Copy Context
Open in Agent
```

。

Agentに渡す実体は、人間向け文章ではなく、

```text
lattice://workspace/current/
  scene/demo
  ?entity=damage
  &t=5.200s
  &rev=8d2af...
```

みたいなlocator。

実際には一行にする。

```text
lattice://scene/demo?entity=damage&t=5.2s&rev=8d2af
```

Agentはこれを見たら、

```bash
lattice inspect \
  'lattice://scene/demo?entity=damage&t=5.2s&rev=8d2af' \
  --json
```

を実行する。

すると、

```json
{
  "scene": "demo",
  "playhead": "5.2s",

  "selection": {
    "id": "damage",
    "kind": "region",

    "canvas_region": [
      "62%",
      "15%",
      "22%",
      "12%"
    ]
  },

  "source": {
    "media": "game",
    "time": "12m40.2s"
  },

  "vel": {
    "file": "scenes/demo.vel",
    "line": 8,
    "column": 3
  }
}
```

まで取れる。

**Pointer自体は小さく、情報はCLIでdereferenceする。**

これはかなり大事だと思う。

---

# 「これ」をAgentが理解できる

例えば人間がCanvasでCalloutを選択。

そのままCoding Agentへ、

> これ、もう少し目立つように

だけ送る。

添付contextとして、

```text
@lattice:pointer/01K...
```

が入る。

Agent：

```bash
lattice inspect @pointer/01K... --json
```

↓

```text
selected:
  demo.damage_callout

at:
  scene 5.2s

source:
  fight @ 12:40.2

generated by:
  callout

VEL:
  demo.vel:14-18
```

まで辿れる。

これならComputer UseでGPUIをクリックさせる必要がない。

---

# CanvasだけじゃなくTimelineからも同じPointer

Timelineなら、

```text
クリックしたClip
+
playhead
+
selected time range
```

。

例えば3秒区間をドラッグ選択して、

> ここテンポ悪い

と渡す。

```json
{
  "scene": "intro",

  "range": {
    "start": "3.2s",
    "end": "6.7s"
  },

  "entities": [
    "intro.gameplay"
  ]
}
```

Agentは、

```bash
lattice inspect range \
  --scene intro \
  --from 3.2s \
  --to 6.7s
```

で、

```text
Media source
Speech words
Placements
Audio
Cuts
```

を調べられる。

つまりPointerの対象はEntityだけじゃなく、

> **時間範囲そのもの**

でもいい。

これは動画編集ではかなり重要。

---

# Preview上の「何もEntityがない場所」も指したい

例えば、

> この右上、寂しい

と指す。

そこにはLattice Entityが存在しない。

だからPointerには、

```text
spatial_region
```

単体を許すべき。

```json
{
  "scene": "demo",
  "playhead": "7.1s",
  "canvas_region": {
    "x": 0.72,
    "y": 0.03,
    "width": 0.24,
    "height": 0.18
  },
  "selection": []
}
```

。

これでAgentは、

> 「右上のこの空間」

をちゃんと理解できる。

**Hit-test結果をPointerの必須条件にしない。**

---

# Source Viewerからも指せる

これはかなり便利そう。

素材ブラウザで、

```text
capture.mp4
12:31.2 ～ 12:34.7
```

を選択。

> この場面をdemoの後に使って

と渡す。

このPointerは、

```text
Project timeline
```

ではなく、

```text
Media source space
```

に属する。

なので、

```rust
enum PointerTarget {
    ProjectEntity(...),
    SceneTime(...),
    SceneRange(...),
    CanvasRegion(...),
    SourceTime(...),
    SourceRange(...),
    SourceRegion(...),
}
```

くらい一般化しておきたい。

---

# Pointerを保存するか

ここは**基本保存しない**でいいと思う。

```text
Studio session
└ current pointer / recent pointers
```

だけ。

ただし人間が、

> ここ後で直す

と永続化したら、それはPointerではなく、

```text
Marker
Annotation
Issue
```

に昇格する。

つまり、

```text
ephemeral Pointer
     ↓ pin
persistent Annotation
```

。

この区別が綺麗。

---

# Revisionは絶対入れる

Coding Agentが処理している間にProjectが変わる可能性がある。

Pointer：

```text
revision = abc123
```

。

現在：

```text
revision = def456
```

だったら、

```bash
lattice inspect pointer ...
```

が、

```text
warning:
pointer was created at revision abc123

current revision: def456

entity still resolves:
  demo.damage

temporal position changed:
  5.2s → 6.1s
```

くらい返す。

Entityが消えていたら、

```text
pointer stale
```

。

**Pointerを黙って現在状態へ解釈し直さない。**

これはかなりLatticeらしいところ。

---

# Identity設計にも影響する

これをやるなら、

```vel
callout fight at 5.2s {
  ...
}
```

みたいな匿名Surface要素にも、内部的にはstable identityが要る。

ただし以前話したように、人間にはIDを書かせなくていい。

Compilerが、

```text
derived-id:
  scene/demo
  source span
  AST identity
```

などからstable-ishなIDを生成する。

そしてGUI編集で永続参照が必要になった瞬間、

```diff
-callout fight at 5.2s {
+callout fight at 5.2s as damage_callout {
```

と明示名を付けられる。

Pointerが外へ出ることは、**stable identityが必要になる代表的な瞬間**ね。

---

# 「Copy Context」はPointerより少し重くする

二種類あるといい。

### Copy Pointer

```text
lattice://...
```

だけ。

AgentがCLIで調べる前提。

### Copy Context

Markdown/JSONとして、

```text
Scene: demo
Playhead: 5.2s
Selection: damage_callout
VEL: scenes/demo.vel:14-18
Source: game @ 12:40.2
```

をClipboardへ。

普通のチャットAgentやissueに貼る用途。

でも正規APIはPointer。

---

# 「Send to Agent」はAgent固有実装にしない

Lattice内に、

```text
Claude
Cursor
Codex
Grok
```

integrationを作り始めたくない。

代わりに例えば、

```text
Open Agent Command
```

を設定可能にする。

ユーザー設定：

```toml
[agent]
command = "cursor-agent"
args = ["--context", "{pointer}"]
```

とか、

```toml
[agent]
command = "claude"
args = ["--append-system-prompt", "{pointer}"]
```

。

あるいは単純に、

```text
Copy Pointer
```

だけ提供してCoding Agent側へ貼らせる。

MCP対応Agentなら将来、

```text
Lattice Studio
   ↓
lattice mcp
   ↓
current-context Resource
```

として、

```text
lattice://session/current
```

を読ませてもいい。

ただし**Pointerという概念自体はCLI/MCP非依存**にする。

---

# AgentからGUIへ逆方向も欲しくなる

これはほぼ確実ね。

Agentが、

> ここを確認して

と返す。

```text
lattice://scene/demo?entity=damage&t=5.2s
```

を出す。

Lattice StudioでそのURIを開けば、

```text
Scene demoへ移動
Playhead 5.2s
damageを選択
Canvasでハイライト
Codeも該当行へ
```

。

つまりPointerは、

```text
Human → Agent
```

だけじゃなく、

```text
Agent → Human
```

にも使える。

これ、かなり強い。

---

# Diagnosticsとも同じ仕組みに統一できる

前にCompiler Diagnosticを、

```text
Source location
Project entity
Temporal range
Spatial region
```

まで持たせたいと言ったでしょう。

だったら、

```text
Diagnostic
  contains EditPointer
```

でいい。

Git semantic diffも、

```text
Diff hunk
  contains before/after pointer
```

。

Annotationも、

```text
Annotation
  anchored by pointer-like reference
```

。

つまり一個の概念が、

```text
GUI selection
Compiler error
Agent context
Semantic diff
Review comment
```

全部を繋ぐ。

これはCore近くに置く価値があるわ。

---

## 私なら名前は `Locus` にするかもしれない

`Pointer`でも十分分かりやすいけれど、Lattice内部用語としては、

```text
Locus
```

が結構合う。

「プロジェクト上の場所」。

```rust
struct Locus {
    revision: Revision,
    entity: Option<EntityRef>,
    temporal: Option<TemporalLocus>,
    spatial: Option<SpatialLocus>,
    source: Option<SourceLocus>,
}
```

。

CLI：

```bash
lattice locus current
lattice locus inspect <uri>
lattice locus open <uri>
```

Studio：

```text
Copy Locus
Point Agent Here
```

Agent：

```text
inspect locus
```

。

私はこれ、**Latticeの一級概念にしてもいいくらい重要**だと思う。

LatticeではCode・Timeline・Canvas・Source Viewerが全部同じProjectの別Projectionになる。その異なるビュー間で「ここ」を共有する抽象化が必要だったのよ。

だから、

> **Locus = Latticeにおける「ここ」**

と定義すると、GUIとCoding Agentを繋ぐだけじゃなく、IDE全体のナビゲーションモデルまで一気に整理できるわ。

---

# you asked

message time: 2026-08-19 13:24:30

実装始めるくらいいいところまでまとまってきたかな

---

# chatgpt response

うん、**実装を始めるには十分まとまった**と思うわ。むしろここから先、紙上で詰め続けると「実装して初めて分かること」を空想で埋め始める段階に入る。

## 事実

少なくとも、最初の縦切りを作るための重要な意思決定はもう揃っている。

- **Lattice**：エコシステム全体
- **VEL**：Rails的・MagicalなSurface DSL。汎用言語にはしない
- **Rust Core**：時間・空間・媒体・配置・Project Graph・validationなど、意味論の最低限
- **Wasm Component**：stdlib、Convention、解析、生成などの拡張境界
- **MoonBit**：Wasm拡張を書く推奨言語。ただしLatticeはMoonBitへ直接依存しない
- **GPUI**：Studio
- **FFmpeg first**：decode/encode/render backend。ただしCore意味論には漏らさない
- **CLI first**：`check / explain / inspect / render`。Agent integrationの正規境界
- **MCP later**：CLI/Coreの薄いadapter
- **モデル内蔵なし**：Coding Agentは完全に外付け
- **Git**：永続履歴。Studioはsession Undo/Redoだけ
- **Workspace = 普通のdirectory/repo**
- **Compile / Resolve / Renderを分離**
- **Magicは許すがExplainableかつMaterialize可能**
- **Quint / PBT / Golden**の役割分担
- **OTIO**：内部IRではなく外向きeditorial interchange
- **Locus**：Code / Timeline / Canvas / Agent間で「ここ」を指す共通概念

これだけ決まっていれば、「何を作っているのか分からなくなって全面書き直し」はかなり避けられる。

逆にまだ決まっていない、

```text
Core IRの厳密な全型
WIT ABIの最終形
VEL文法の細部
Render Graph
キャッシュ方式
GPUI Timeline実装詳細
OTIO mapping
Extension packaging
```

あたりは、**実装前に決め切る必要がないもの**よ。

特にWITとCore IRは、実際に2〜3個のstdlib operationを通してから固めた方がいい。

---

## 推測（示唆）

今は「製品実装」を始めるというより、**LatticeのArchitecture Walking Skeletonを作る段階**と捉えるのが一番よさそう。

最初の成功条件を、私はかなり小さくする。

```text
main.vel
   ↓
parse
   ↓
Magic / Convention
   ↓
Wasm stdlib
   ↓
Core IR
   ↓
validate
   ↓
FFmpeg
   ↓
preview.mp4
```

これが一本通ればいい。

題材も今まで使ったこれで十分。

```vel
project "demo"

convention commentary

media game "capture.mp4"

sequence main {
  demo
}

scene demo {
  game[10s..20s] as fight

  freeze fight at 5.2s for 1.5s

  title "Hello" {
    at 2s for 3s
  }
}
```

この程度を本当に動画へする。

最初からTTS、Beat解析、Agent、OTIO、巨大Timelineなんて入れない。

ただしArchitecture上は、

```text
freeze → Wasm
title  → Wasm
convention → Wasm
```

を守る。

ここでRustに、

```rust
match command {
    "freeze" => ...
    "title" => ...
}
```

と書き始めないことの方が重要。

---

そして第一マイルストーンではStudioすら、かなり薄くていいと思う。

```text
┌─────────────────────────────────────┐
│ VEL Editor          Preview         │
│                                     │
│ scene demo {         [ video ]      │
│   ...                               │
│ }                                   │
├─────────────────────────────────────┤
│ Problems │ Explain                  │
└─────────────────────────────────────┘
```

これだけ。

Timelineは後でいい。

まず、

> VELを書く → 保存 → compile → preview更新 → diagnostics表示

までGPUIで通す。

この時点でLatticeの「IDEとしての生命」がある。

---

その次の段階で初めて、

```text
Canvas selection
↓
Locus
↓
該当VELへジャンプ
```

を作る。

さらに、

```text
Copy Locus
↓
Coding Agent
↓
lattice inspect <locus>
```

まで通せば、**Lattice独自の編集体験**がもう一つ成立する。

ここまで来たらTimelineを作る価値も実データから判断できる。

---

私はむしろ今、**仕様書を増やすよりリポジトリを作った方が情報量が増える地点**だと思う。

最初のcrate構成も仮で十分。

```text
lattice/
├─ crates/
│  ├─ lattice-core
│  ├─ lattice-vel
│  ├─ lattice-wasm
│  ├─ lattice-media
│  ├─ lattice-cli
│  └─ lattice-studio
│
├─ wit/
├─ stdlib/
│  └─ temporal/
├─ spec/
├─ tests/
│  ├─ golden/
│  ├─ properties/
│  └─ model/
└─ examples/
   └─ gameplay-commentary/
```

最初のstdlibは本当に、

```text
freeze
title
flow
```

くらいでいい。

この3つだけでも、

- 時間変換
- Canvas生成
- 時間配置

の三種類を通せるので、Wasm ABIの設計をかなり殴れる。

そしてそこでABIが気持ち悪ければ**即壊す**。今なら壊すコストが安い。

だから答えとしては、**「実装を始めてもいい」より「そろそろ実装を始めた方が設計が進む」**くらいまで来ていると思うわ。今後の設計会話も、動いているWalking Skeletonを横に置いてやった方がずっと鋭くなる。

---

# you asked

message time: 2026-08-19 13:28:12

プロジェクト初期化に必要なことをまとめておく

---

# chatgpt response

そうね。**「今日リポジトリを切って、Coding Agentに最初の実装を投げられる状態」**までに必要なものを一度固定しておくといいわ。今は完成形の設計書より、壊しやすいWalking Skeletonを作るための初期条件が重要。

## 事実

現時点で初期化時に前提として置いてよさそうなのはこれ。

```text
Lattice
├─ Rust          Core / Compiler / Host / CLI / Studio
├─ GPUI          Studio UI
├─ VEL           Rails的Surface DSL
├─ Wasm Component 拡張境界
├─ MoonBit       推奨Extension実装言語
├─ FFmpeg        最初のMedia/Render backend
├─ Git           永続履歴
├─ Quint         状態遷移仕様
├─ PBT           型・変換・代数則
└─ Golden        Surface semantics / Explain / Diagnostics
```

そして最初の縦切りは、

```text
.vel
 ↓
parse
 ↓
invocation AST
 ↓
Wasm stdlib / convention
 ↓
Core IR
 ↓
validate
 ↓
Render Plan
 ↓
FFmpeg
 ↓
preview
```

これ一本を通すこと。

---

# 初期ディレクトリ

私なら最初はこうする。

```text
lattice/
├─ Cargo.toml
├─ rust-toolchain.toml
├─ README.md
├─ LICENSE
├─ .gitignore
│
├─ crates/
│  ├─ lattice-core/
│  ├─ lattice-vel/
│  ├─ lattice-wasm/
│  ├─ lattice-media/
│  ├─ lattice-cli/
│  └─ lattice-studio/
│
├─ wit/
│  └─ lattice/
│
├─ stdlib/
│  ├─ temporal/
│  ├─ visual/
│  └─ convention-commentary/
│
├─ spec/
│  ├─ build_protocol.qnt
│  └─ resolution.qnt
│
├─ tests/
│  ├─ golden/
│  │  ├─ compile/
│  │  ├─ explain/
│  │  ├─ diagnostics/
│  │  └─ render-plan/
│  └─ fixtures/
│
├─ examples/
│  └─ gameplay-commentary/
│     ├─ main.vel
│     └─ assets/
│
└─ docs/
   ├─ architecture.md
   ├─ principles.md
   └─ glossary.md
```

最初からcrateを細分化しすぎる危険はあるので、`lattice-core / lattice-vel / lattice-cli`だけで始めて後から分けても構わない。

---

# 最初に書く3ドキュメント

これはCoding Agent対策としても有効。

### `docs/principles.md`

10〜15行でいい。

```text
- Magic is allowed, hidden behavior is not.
- Every magic expansion must be explainable.
- VEL is a DSL, not a general-purpose language.
- Extension execution boundary is Wasm Component/WIT.
- MoonBit is recommended, not required.
- Rust Core owns semantic primitives.
- Wasm produces plans/IR; it is not in the render hot path.
- Compile, Resolve, and Render are separate phases.
- Project state is text-first and Git-friendly.
- Persistent history belongs to Git.
- FFmpeg is a backend, not the semantic model.
- External coding agents interact primarily through CLI.
```

これを最初に置いておくとAgentが「便利なのでOpenAI SDKを追加しました」とかやりにくくなる。そういう善意は早めに封じた方がいいわ。

### `docs/architecture.md`

最初は図一枚。

```text
VEL → Parser → AST
               ↓
          Wasm lowering
               ↓
            Core IR
               ↓
           Validator
               ↓
          Render Plan
               ↓
            FFmpeg
```

GPUI/CLIはCoreのclient。

### `docs/glossary.md`

ここは地味に重要。

```text
Project
Workspace
Sequence
Scene
Source
Placement
Composition
TimeMap
Locus
Convention
Resolve
Core IR
Render Plan
```

同じものをAgentごとに別名で実装される事故を防ぐ。

---

# Rust workspace

最初からworkspace-wideの設定は揃える。

```toml
[workspace]
resolver = "3"

members = [
  "crates/lattice-core",
  "crates/lattice-vel",
  "crates/lattice-wasm",
  "crates/lattice-cli",
  "crates/lattice-studio",
]
```

そしてworkspace dependencyで、

```text
serde
thiserror
tracing
```

あたりを共有。

ただしCoreについては、**依存をかなり少なく保つ**方針にしたい。

特に、

```text
GPUI
FFmpeg bindings
Wasmtime
```

を`lattice-core`へ漏らさない。

依存方向は、

```text
core
 ↑
vel
 ↑
wasm
 ↑
cli / studio
```

くらいに保つ。

---

# Toolchainを固定

バージョン番号自体は後で決めればいいけど、

```text
rust-toolchain.toml
Cargo.lock
MoonBit toolchain
WIT tooling
```

は最初から固定する。

Wasm周りはまだ動きが速いので、

> 「最新版なら動く」

を開発環境の仕様にしない方がいい。

MoonBitについても、Extensionの再現性まで考えるなら、

```text
compiler version
SDK version
WIT ABI version
```

を記録できる形にする。

---

# WITは最初からversioned

最初のWITは万能Plugin APIにしない。

例えば、

```text
wit/lattice/
├─ core-types.wit
└─ lowering.wit
```

程度。

最初の目的は、

```text
VEL Invocation
   ↓
Wasm
   ↓
Core Fragment
```

を通すこと。

`ProjectContext`を丸ごと渡すようなAPIは避ける。

最初に通す語彙は、

```text
freeze
title
flow
```

の3つがちょうどいいと思う。

それぞれ、

- `freeze` → temporal
- `title` → visual/composition
- `flow` → placement

と違う方向を殴れるから。

---

# Core IR v0

完成モデルは決めない。

でも最初に必要な型だけ置く。

```rust
Project
Sequence
Scene

MediaRef
StreamRef

Time
Duration
TimeSpan
TimeMap

Placement
Visual
Audio

Region
Transform

Property<T>
Curve<T>

Provenance
Diagnostic
```

ポイントは、**IRをserde JSONでdumpできるようにする**こと。

```bash
lattice compile main.vel --emit-ir
```

で即見られる。

初期開発ではこれがものすごく便利。

---

# VEL Parser

最初から完全なgrammarを作らない。

最低限、

```text
declaration
invocation
block
string
identifier
unit literal
range
binding/as
modifier
```

だけ。

つまり前に作ったprototypeの思想をそのままRustへ移す。

```vel
freeze fight at 5.2s for 1.5s
```

を、

```rust
Invocation {
    name: "freeze",
    args: [...],
    modifiers: {...},
}
```

へ落とせれば十分。

`freeze`自体の意味はParserへ入れない。

---

# Source mapは初日から

これは後付けすると痛い。

```text
VEL source span
   ↓
Invocation
   ↓
Wasm expansion
   ↓
Core IR node
```

を追跡する。

最低限、

```rust
struct Provenance {
    source_span: Option<SourceSpan>,
    origin: Origin,
}
```

をCore nodeへ付ける。

これが、

```text
Diagnostics
Explain
Locus
Semantic diff
Agent context
```

全部の土台になる。

---

# CLIのv0

最初は4コマンドでいい。

```bash
lattice check main.vel
lattice compile main.vel --emit-ir
lattice explain main.vel
lattice render main.vel -o preview.mp4
```

そして**全部`--json`対応を前提に設計**。

Coding Agentが最初から使える。

Studio専用APIを先に作る必要がない。

---

# `explain`は最初のDefinition of Doneに入れる

これは意外と重要。

例えば、

```vel
scene intro over narration.intro {
  game[10s..20s]
}
```

に対して、

```text
Bare media expanded by commentary convention:

video:
  source game[10s..20s]
  placement canvas-fill

audio:
  source game.audio
  gain -15dB
```

くらい出す。

Magicを導入してからExplainを足すのでは遅い。

**MagicとExplainは同じfeature**と考えた方がいい。

---

# Rendering

最初は欲張らない。

最初のbackendが対応するのは、

```text
media slice
freeze
text/title
basic composition
audio passthrough
```

くらい。

Codec選択やColor管理、高度Effectは全部後。

最初のゴールは、

> 10秒の動画が正しい時間でfreezeして、文字が載る

それで十分。

---

# GPUI Studio

最初のStudioはこれでいい。

```text
┌────────────────────────────┐
│ Code          │ Preview    │
│               │            │
├───────────────┴────────────┤
│ Problems │ Explain         │
└────────────────────────────┘
```

必要なもの：

```text
VEL editor
Save
Compile
Preview frame/video
Diagnostics
Explain
```

Timelineはまだ作らない。

Canvas直接編集もLocusも後。

まずCompilerとの循環を作る。

---

# Testingを初日から置く

特にgolden。

```text
tests/golden/compile/basic-scene/
├─ input.vel
├─ expected.ir.json
└─ expected.explain.txt
```

第一号はゲーム解説サンプル。

PBTはまず、

```text
TimeMap
parse → format → parse
```

。

Quintはコードを書くより先に巨大仕様を作る必要はない。

最初は、

```text
Compiled
Resolved
Planned
```

程度のBuild lifecycleモデルだけ空ファイルでも置いておけばいい。

---

# CI

最低限、

```text
cargo fmt --check
cargo clippy
cargo test
golden tests
```

。

Wasm stdlibが入ったら、

```text
build stdlib components
component tests
```

追加。

Quintは仕様が一つでも動き始めたら、

```text
quint verify
```

をCIへ。

---

# Git hooksは軽め

絶対必須ではないけれど、

```text
fmt
fast tests
```

くらい。

動画生成をpre-commitに入れるような蛮行はしない。

Render goldenなど重いものはCI。

---

# Sample projectは最初からrepo内に置く

これ、実装の判断基準になる。

```text
examples/gameplay-commentary/main.vel
```

は我々が今まで書いていたゲーム解説動画を縮小したもの。

新しいAPIを入れるたび、

> このexampleは良くなったか、悪くなったか

を見る。

LatticeのRails的な「気持ちよさ」を守る一番簡単なbenchmarkになる。

---

## 最初のコミット群

私ならかなり意図的に分ける。

```text
1. chore: initialize Rust workspace
2. docs: add Lattice design principles
3. feat(core): introduce minimal semantic IR
4. feat(vel): parse generic invocation DSL
5. feat(wasm): load lowering component
6. feat(stdlib): implement freeze component
7. feat(cli): add check and compile
8. feat(explain): track expansion provenance
9. feat(media): render basic FFmpeg timeline
10. feat(studio): add GPUI shell
```

こうするとCoding Agentに途中から投げても、歴史が読みやすい。

---

## 推測（示唆）

**初期化時に決めない方がいいもの**もリストにしておきたい。

```text
× 完成したVEL grammar
× 完成したCore IR
× Extension marketplace
× Plugin signing
× GPUI Timeline architecture
× GPU renderer
× Collaboration
× OTIO完全mapping
× TTS provider一覧
× Cache最適化
× Distribution/installer
```

この辺を初期化Issueに混ぜると、一気に大プロジェクトらしい重力が発生する。

今必要なのは、

> **DSLのMagicがWasmを通ってCore IRになり、説明でき、10秒の動画として出る。**

これだけ。

これを`examples/gameplay-commentary`で一本通した時点を**Lattice Milestone 0**にしてしまうのがいいと思うわ。

そのWalking Skeletonさえ生きれば、以降の設計議論は全部「実物を壊して確かめる」に移れる。今の段階なら、その方が圧倒的に速い。

---

# you asked

message time: 2026-08-19 13:33:02

初期プラットフォームどうしようかな。Windowsでゲームはやるけど

---

# chatgpt response

ここは「作りやすいOS」より、**最初に毎日dogfoodできるOS**を優先した方がいいと思うわ。ゲーム素材・NVENC・実ファイル運用まで含めると、Windows-firstにはかなり強い理由がある。

うん。**私は初期ターゲットを Windows 11 x64 に振っていいと思う**。ゲーム実況を最初のdogfoodにするなら、ここでmacOS-firstにするメリットより「録画したそのPCで即Latticeを使える」メリットの方が大きいわ。

## 事実

GPUIについては、現在Windowsがちゃんとplatform backendの対象になっている。`gpui_platform`はWindowsではWin32でwindowing、DirectWriteでtextを扱い、Windows用に追加featureを有効化する必要もない。GPUI自体は依然pre-1.0なのでbreaking changeは覚悟がいる。([GitHub](https://github.com/zed-industries/zed/blob/main/crates/gpui/README.md))

Zed本体も現在Windows 11 22H2以降を正式サポートし、GPU要件はDirectX 11。つまり少なくともGPUIのWindows経路は「一応動く実験版」という段階からはかなり進んでいる。([Zed](https://zed.dev/docs/installation))

一方、WindowsではGPUメモリがMacのunified memoryとは違うことや、ファイル操作・path convention・WSL周辺などWindows固有問題が実際に出ている。Zed自身もWindows移植で、DirectX backend、VRAM使用量、ファイル置換、path conventionなどに個別対応してきた。([Zed](https://zed.dev/blog/windows-progress-report))

## 推測（示唆）

私は初期サポート表をこう切る。

| 層 | v0サポート |
|---|---|
| Lattice Core | Windows + Linux CI |
| VEL/compiler | Windows + Linux CI |
| Wasm host | Windows + Linux CI |
| CLI | Windowsを主対象、Linuxも動かす |
| Studio/GPUI | **Windows 11 x64のみ正式対象** |
| Render | Windows native FFmpeg |
| macOS | 後回し |
| Linux Studio | 後回し |
| WSL | **非サポート** |

つまり、**製品はWindows-first、ArchitectureはWindows-onlyにしない**。

これが一番バランスいい。

### OSの最低ラインも狭くていい

GPUI/ZedはWindows 10まで対応しているけれど、Lattice v0でそこまで背負う理由はない。([Zed](https://zed.dev/docs/installation))

私は、

```text
Supported:
  Windows 11 22H2+
  x86_64
```

だけ宣言する。

ARM64もWindows 10も、動いたらラッキー程度。

ゲーム動画編集用の個人プロジェクトの初期段階で、Windows SKU考古学を始める必要はないわ。

---

### 開発環境もWindows native

ここも私は割り切る。

```text
Windows 11
├ Rust MSVC
├ GPUI
├ Wasmtime
├ FFmpeg.exe
├ Git
├ MoonBit toolchain
└ Lattice
```

**WSLを主要開発・実行経路にはしない。**

Latticeが扱う素材が、

```text
D:\Captures\Warframe\...
```

みたいなWindows native fileで、StudioもWin32、FFmpegもWindows processになる以上、

```text
/mnt/d/Captures/...
```

との相互変換をわざわざ最初から問題にする意味がない。

ZedのWindows開発でもWSLとWindows側のpath convention差が独立した課題になっている。([Zed](https://zed.dev/blog/windows-progress-report))

Coding AgentがWSL側にいたとしても、

```bash
lattice.exe check ...
```

をどう呼ぶかは後で解けばいい。

---

# ただしCoreではWindows pathを文字列にしすぎない

これは最初から意識したい。

悪いCore：

```rust
struct Media {
    path: String,
}
```

より、

```rust
struct MediaLocator {
    // ...
}
```

として、

```text
file locator
URL
generated artifact
```

くらい抽象化する。

Windowsでは、

```text
C:\foo
D:\bar
\\server\share
Unicode filenames
```

などがあるから、Core内部で`/`前提のpath manipulationをしない。

RustならOS pathは`Path/PathBuf`で扱って、VEL serializationとの境界でURI/portable representationを考える。

ここはWindows-firstだからこそ早めに痛みを踏める。

---

# RenderingもWindowsで最初に殴る価値が高い

ゲーム実況を本当に作るなら、

```text
1440p60
長尺H.264/H.265
高ビットレート録画
GPU搭載PC
```

みたいな素材が最初から来る。

これはむしろ歓迎したい。

「小さなテストMP4だけでは綺麗に動いたのに、OBSの2時間録画で全部死んだ」が動画編集ソフトの本番でしょう。

ただしv0では、

```text
hardware decode/encode required
```

にはしない。

まず、

```text
FFmpeg software path = correctness baseline
```

を作る。

その上に、

```text
NVENC
AMD
Intel
```

などhardware accelerationをBackend capabilityとして足す。

特定GPUをLattice semanticsへ混ぜない。

---

# PreviewでGPUを二重に考えない

ここも重要そう。

```text
GPUI GPU rendering
+
Video decode/render
```

は別問題。

v0では、

```text
FFmpeg
  ↓ decode frame
CPU/RAM frame
  ↓ upload
GPUI
```

くらいの間抜けでもいい。

最初から、

```text
D3D11 shared texture
zero-copy decode
hardware surfaces
```

をやり始めると、Walking SkeletonがWindows graphics API研究所になる。

それはPreview performanceが本当に問題になってからで十分。

ただ、Windows-firstなら後でDirectX周辺を最適化する道は自然にある。Zed/GPUIもWindowsではDirectX 11 backendへ移行している。([Zed](https://zed.dev/blog/windows-progress-report))

---

# 私ならCIは逆にLinuxも初日から入れる

ここがちょっと逆説的。

実機開発：

```text
Windows
```

CI：

```text
Windows
Linux
```

。

理由は、

```text
lattice-core
lattice-vel
lattice-wasm
```

に、

```rust
use windows::Something;
```

がうっかり入り込んだ瞬間にLinux CIが殴ってくれるから。

一方、

```text
lattice-studio
lattice-media-windows
```

はWindows-onlyでも構わない。

将来macOSへ行きたくなったら、

```text
Core = ほぼそのまま
VEL = そのまま
Wasm components = そのまま
```

で、

```text
Media driver
Studio platform edge
Packaging
```

だけ増やせる状態を狙う。

---

# 最初のplatform abstraction

あまり大げさなHALはいらない。

これくらいでいい。

```text
lattice-core
  OS非依存

lattice-media
  共通trait/API

lattice-media-ffmpeg
  FFmpeg backend

lattice-studio
  GPUI

lattice-platform
  filesystem / process / platform quirks
```

もしくは`platform` crateすら作らず、

```rust
#[cfg(windows)]
```

をadapterの端にだけ置く。

「将来クロスプラットフォームだから」という理由で巨大Platform Abstraction Layerを先に書くのは、未来へのお供え物になりがちだからね。

---

## 初期プラットフォーム方針を一文にするなら

> **Lattice v0 is developed and dogfooded on Windows 11 x64. Core, VEL, and Wasm interfaces remain platform-neutral and are continuously tested on Linux. Other Studio platforms are explicitly deferred.**

これでいいと思う。

そしてLatticeの場合、Windows-firstは単なるユーザー都合じゃない。

**ゲーム録画・巨大Media・離散GPU・VRAM制約・Windows path・hardware encodingという面倒な現実を最初から食らえる**。後でWindows portするより、動画編集ソフトとしてはむしろ健全な順番だと思うわ。

---

# you asked

message time: 2026-08-19 13:39:06

今見えてる依存関係を整理してみる

---

# chatgpt response

そうね。今のLatticeはかなり層が増えてきたから、**「何が何に依存してよいか」**を一度固定した方がいいわ。ここを曖昧にすると、数週間後には`lattice-core`がFFmpegとGPUIとWasmtimeを全部importする怪物になる。

## 事実

今見えている依存関係を、一番大きく描くとこう。

```text
                     ┌─────────────────┐
                     │ Lattice Studio  │
                     │   Rust + GPUI   │
                     └────────┬────────┘
                              │
                     ┌────────▼────────┐
                     │   Lattice CLI   │
                     └────────┬────────┘
                              │
              ┌───────────────┼────────────────┐
              │               │                │
      ┌───────▼───────┐ ┌────▼─────┐  ┌──────▼──────┐
      │ lattice-vel   │ │lattice-wasm│  │lattice-media│
      │ parser/lower  │ │ Wasmtime   │  │ FFmpeg      │
      └───────┬───────┘ └────┬──────┘  └──────┬──────┘
              │               │                │
              └───────────────┼────────────────┘
                              ▼
                     ┌─────────────────┐
                     │  lattice-core   │
                     │ Semantic IR     │
                     └─────────────────┘
```

ただ、これは少し誤解を生む。

**CLIはStudioの下ではない**。両方ともCore周辺サービスのclient。

正確にはこうね。

```text
                          Applications
                ┌────────────┴────────────┐
                │                         │
          lattice-studio              lattice-cli
             GPUI                        CLI
                │                         │
                └────────────┬────────────┘
                             │
                          Services
          ┌──────────────────┼─────────────────┐
          │                  │                 │
    lattice-vel        lattice-wasm      lattice-media
          │                  │                 │
          └──────────────────┼─────────────────┘
                             │
                             ▼
                        lattice-core
```

**Coreが一番下。**

---

# 1. `lattice-core`

ここは最重要。

依存してよいものは極力、

```text
Rust std
serde
small utility crates
```

程度。

持つもの：

```text
Identity
Time
Duration
TimeMap

CoordinateSpace
Point
Region
Transform

MediaRef
StreamRef

Sequence
Scene
Placement
Composition

Visual
Audio

Property<T>
Curve<T>
Signal<T>

EventSet
SpanSet
Collection

Project Graph
Provenance
Diagnostics
Locus
```

そして、

```text
lattice-core
   ✗ GPUI
   ✗ FFmpeg
   ✗ Wasmtime
   ✗ MoonBit
   ✗ VEL parser
   ✗ Git
```

これを鉄則にしたい。

Coreは、

> **Latticeの意味論だけ**

知っている。

---

# 2. `lattice-vel`

依存：

```text
lattice-vel
    ↓
lattice-core
```

持つもの：

```text
lexer/parser
Surface AST
Invocation AST
unit literals
source map

VEL → invocation lowering
VEL diagnostics
formatter
```

ただし、

```text
freeze
callout
replay
commentary convention
```

の意味は知らない。

例えば、

```vel
freeze fight at 5.2s for 1.5s
```

を、

```text
Invocation {
  name: "freeze"
  ...
}
```

までにする。

その先はWasm/stdlib。

---

# 3. `lattice-wasm`

依存：

```text
lattice-wasm
   ├→ lattice-core
   └→ Wasmtime
```

必要ならVELのInvocation representationも受けるので、

```text
lattice-wasm
   → lattice-vel?
```

になりそうだけれど、私は**避けたい**。

代わりにInvocationを、

```text
lattice-core
```

とは別の小さな、

```text
lattice-dsl-api
```

に切る手がある。

例えば、

```text
lattice-dsl-api
├ Invocation
├ Argument
├ Modifier
└ SourceSpan
```

。

すると、

```text
lattice-vel ────────┐
                    ▼
              lattice-dsl-api
                    ▲
lattice-wasm ───────┘
```

になり、VEL parserとWasm hostが直接依存しない。

これはかなりおすすめ。

---

# 4. Wasm Component側

ここは逆方向の依存を明確にする。

```text
stdlib component
     │
     │ WIT
     ▼
Lattice Host API
```

つまりComponentはRust crateへ依存しない。

依存するのは**WIT contractだけ**。

```text
MoonBit
Rust
Go
...
  ↓ compile
Wasm Component
  ↓ WIT
Lattice
```

。

公式MoonBit SDKを作るなら、

```text
lattice-sdk-moonbit
      ↓
Lattice WIT definitions
```

だけ。

Lattice本体：

```text
✗ MoonBit compiler dependency
✗ MoonBit runtime dependency
```

でもいい。

開発ツールとしてMoonBit CLIが必要なだけ。

---

# 5. `lattice-media`

ここは少し分割したくなる。

概念API：

```text
lattice-media
    ↓
lattice-core
```

FFmpeg実装：

```text
lattice-media-ffmpeg
    ├→ lattice-media
    └→ FFmpeg
```

の方が綺麗。

例えば、

```rust
trait MediaBackend {
    fn probe(...);
    fn frame(...);
    fn render(...);
}
```

を`lattice-media`。

実装を、

```text
lattice-media-ffmpeg
```

。

そうすれば後で、

```text
lattice-media-native
lattice-media-windows
```

などを足せる。

---

# 6. Render Plannerも独立させる価値がある

今の設計では、

```text
Core IR
↓
Render Plan
↓
FFmpeg
```

だから、

```text
lattice-render
   ↓
lattice-core
```

を置いてもいい。

```text
lattice-render
├ RenderPlan
├ validation
├ graph lowering
└ backend-neutral operations
```

。

そして、

```text
lattice-render-ffmpeg
   ├→ lattice-render
   └→ lattice-media-ffmpeg
```

。

つまり、

```text
Core Semantic IR
       ↓
lattice-render
       ↓
Backend-neutral RenderPlan
       ↓
lattice-render-ffmpeg
```

。

これは結構きれい。

---

# 7. `lattice-studio`

依存が一番多い。

```text
lattice-studio
├ lattice-core
├ lattice-vel
├ lattice-wasm
├ lattice-media
├ lattice-render
└ GPUI
```

ただしStudio自身に、

```text
FFmpeg command generation
VEL semantic lowering
Wasm execution semantics
```

を実装させない。

Studioはあくまで、

> **UI adapter**

。

例えばPreviewも、

```text
Studio
↓
RenderService
↓
Render Plan/backend
```

。

---

# 8. CLI

CLIも同じく薄く。

```text
lattice-cli
├ lattice-core
├ lattice-vel
├ lattice-wasm
├ lattice-render
└ lattice-media
```

。

提供：

```text
check
compile
explain
inspect
resolve
render
```

。

将来、

```text
lattice mcp
```

を入れるなら、

```text
lattice-mcp
    ↓
lattice-service/API
```

としてCLI binaryに統合してもいい。

---

# 9. `lattice-service`を置くか

ここ、今の段階だとかなり有望。

StudioとCLIが同じ処理を使うから、

```text
lattice-service
```

または、

```text
lattice-engine
```

を置く。

例えば、

```rust
Engine {
    compile()
    check()
    resolve()
    explain()
    render()
    inspect()
}
```

。

依存は、

```text
lattice-engine
├ lattice-core
├ lattice-vel
├ lattice-wasm
├ lattice-render
└ lattice-media
```

。

上位は、

```text
lattice-cli ───────┐
                   ▼
             lattice-engine
                   ▲
lattice-studio ────┘
```

。

これ、かなり良いと思う。

CLIとGPUIでロジックが分裂するのを防げる。

---

# 10. Git

Gitはさらに上。

```text
lattice-git
```

をもし作るなら、

```text
lattice-git
   ↓
lattice-core
```

くらい。

やることは、

```text
semantic diff
history projection
current revision
workspace dirty state
```

。

**Core IR自体がGitを知らない。**

Locusがrevisionを持つとしても、

```text
RevisionId
```

というopaque valueで十分。

`git commit hash`という意味はGit adapter側に置く。

---

# 11. OTIO

これも外向きadapter。

```text
lattice-otio
    ↓
lattice-core
```

。

理想は、

```text
Core IR
 ↕ projection/import
OTIO
```

。

だからRender/Studio/VELとは直接依存させない。

必要ならCLIが、

```text
lattice-cli
→ lattice-otio
```

を呼ぶ。

---

# 12. Quint

Quintは**production dependencyではない**。

```text
spec/*.qnt
     │
     ↓ model traces
tests/model-based
     │
     ↓
lattice-engine/core
```

。

つまり、

```text
Quint
  testing/tooling only
```

。

Core crateからQuint runtimeを呼ぶ必要はない。

---

# 13. PBT

同じくdev dependency。

```text
proptest
   ↓
lattice-core
lattice-vel
...
```

。

特に、

```text
core:
  TimeMap
  Geometry
  Patch

vel:
  parser/formatter/desugar
```

を殴る。

---

# 14. Golden

これもtestsから各境界を見る。

```text
Golden
├ VEL → Invocation
├ Invocation → Core IR
├ Convention → Core IR
├ Explain
├ Diagnostics
├ Core IR → Render Plan
├ Render Plan → FFmpeg
└ OTIO import/export
```

依存方向はproduction codeとは逆の**観測**だから、アーキテクチャ依存には入れなくていい。

---

# 15. FFmpeg

FFmpegとの接点はできるだけ二箇所以下に抑えたい。

```text
lattice-media-ffmpeg
lattice-render-ffmpeg
```

。

最終的に統合してもいい。

```text
lattice-ffmpeg
```

一crateにして、

```text
probe
decode
encode
filtergraph
```

全部まとめる方が初期は楽かもしれない。

大事なのは、

```text
FFmpeg
↑
adapter
↑
RenderPlan / Media API
```

で止めること。

---

# 16. GPUI

同様に、

```text
GPUI
↑
lattice-studio
```

で止める。

GPUIの、

```text
Entity<T>
Window
App
Element
```

などをCore型へ漏らさない。

---

# 17. Windows

これもdependencyとして見ると分かりやすい。

```text
Windows APIs
     ↑
platform/media/studio edge
```

。

Core / VEL / Wasm ABIは、

```text
Windows ✗
```

。

初期のWindows-onlyコードは、

```text
lattice-studio
lattice-ffmpeg
lattice-platform-windows
```

あたりに閉じ込める。

---

## 推測（示唆）

ここまでをcrate graphとしてまとめるなら、私は今のところこれが一番好き。

```text
                         ┌──────────────────┐
                         │ lattice-studio   │
                         │      GPUI        │
                         └────────┬─────────┘
                                  │
                         ┌────────▼─────────┐
                         │  lattice-engine  │◄──── lattice-cli
                         └────────┬─────────┘
                                  │
          ┌───────────────────────┼────────────────────────┐
          │                       │                        │
  ┌───────▼────────┐     ┌────────▼───────┐      ┌────────▼───────┐
  │ lattice-vel    │     │ lattice-wasm   │      │ lattice-render │
  └───────┬────────┘     └────────┬───────┘      └────────┬───────┘
          │                       │                        │
          ▼                       ▼                        ▼
  ┌────────────────┐     ┌────────────────┐       ┌────────────────┐
  │lattice-dsl-api │     │ Wasmtime / WIT │       │ lattice-ffmpeg │
  └────────┬───────┘     └────────┬───────┘       └────────┬───────┘
           │                      │                         │
           └──────────────────────┼─────────────────────────┘
                                  ▼
                         ┌──────────────────┐
                         │   lattice-core   │
                         └──────────────────┘
```

厳密には`lattice-ffmpeg → core`ではなく`render/media API`へ依存させたいけれど、概念的にはこれ。

そして横に、

```text
lattice-git  ─→ core
lattice-otio ─→ core
```

。

Wasm側は、

```text
MoonBit SDK
    ↓
WIT
    ↓
Wasm Component
    ↓
lattice-wasm host
```

。

---

## 依存ルールを文章化すると

私はこの8個をArchitecture testにしてもいいと思う。

1. `lattice-core`はLattice内の他crateに依存しない。
2. `lattice-core`はGPUI / Wasmtime / FFmpegに依存しない。
3. `lattice-vel`はWasmやFFmpegを知らない。
4. `lattice-wasm`はVEL parserを知らない。
5. Wasm ComponentはRust内部型を直接知らず、WITのみを契約にする。
6. Render backendはSurface VELを知らない。
7. StudioとCLIは業務ロジックを所有せず、Engineを呼ぶ。
8. OS固有コードはCoreへ侵入しない。

これさえ守れば、内部のcrate数は後から増減していい。

一番注意したいのは、**`lattice-engine`を便利箱にしすぎること**ね。

Engineはorchestration専用。

```text
compile
→ resolve
→ validate
→ render
```

の順番を管理する。

`TimeMap`の意味や`freeze`実装やFFmpeg引数生成までEngineに書き始めたら黄色信号。

---

かなり短く圧縮すると、今のLatticeには**三種類の依存**がある。

```text
Semantic dependency
  → lattice-core

Extension dependency
  → WIT / Wasm Component

Execution dependency
  → FFmpeg / GPUI / OS
```

この三つを混ぜないことが、今のArchitectureのほぼ全部だと思うわ。

この状態なら、次にリポジトリを切る際は、まず `core / dsl-api / vel / wasm / engine / cli` の6個程度から始めて、FFmpegとGPUIはWalking Skeletonが必要になった段階で足しても綺麗に伸ばせそう。