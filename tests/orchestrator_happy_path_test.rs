mod fakes;

use conch::interview::orchestrator::{EndSignal, Orchestrator, OrchestratorConfig};
use conch::interview::tui::state::{AppState, Status};
use conch::interview::tui::UserEvent;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, RwLock};

#[tokio::test]
async fn opening_then_one_turn_then_voice_command_end() {
    // --- set up fakes ---

    // FakeStt scripts: first open_stream is for the first user turn (normal text),
    // second open_stream is for the "that's a wrap" end command.
    let stt = Arc::new(fakes::FakeStt::new(vec![
        vec!["i built this to learn audio".to_string()],
        vec!["that's a wrap".to_string()],
    ]));

    let tts = Arc::new(fakes::FakeTts);

    // FakeLlm replies:
    // 1. Opening (call_llm without tools)
    // 2. Continuation after first user turn (call_llm_with_tools)
    // 3. Closing after voice command
    let llm = Arc::new(fakes::FakeLlm::new(vec![
        "Welcome! Tell me about your project.".to_string(),
        "That's fascinating. What inspired you?".to_string(),
        "Great chat. Thanks for sharing!".to_string(),
    ]));

    let sink = fakes::FakeSink::new();
    let collected = sink.collected();

    let state = Arc::new(RwLock::new(AppState::new(
        "Test Session".to_string(),
        "A test brief".to_string(),
    )));

    let (event_tx, event_rx) = mpsc::channel::<UserEvent>(16);

    let config = OrchestratorConfig {
        model: "fake-model".to_string(),
        brief: "A test brief about audio stuff".to_string(),
        brand: None,
        max_tokens: 256,
        sample_rate: 16_000,
    };

    let orch = Orchestrator::new(
        llm,
        stt,
        tts,
        Box::new(sink),
        state.clone(),
        event_rx,
        config,
    );

    // Spawn the orchestrator in a task; we'll drive it via events.
    let handle = tokio::spawn(async move { orch.run().await });

    // Give the orchestrator time to complete the opening turn.
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    // --- First user turn: toggle mic on, then off ---
    event_tx.send(UserEvent::MicToggle).await.unwrap(); // mic open
    tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    event_tx.send(UserEvent::MicToggle).await.unwrap(); // mic close
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    // --- Second user turn: "that's a wrap" ---
    event_tx.send(UserEvent::MicToggle).await.unwrap(); // mic open
    tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    event_tx.send(UserEvent::MicToggle).await.unwrap(); // mic close

    // Wait for the orchestrator to finish.
    let result = tokio::time::timeout(std::time::Duration::from_secs(5), handle)
        .await
        .expect("orchestrator timed out")
        .expect("orchestrator panicked")
        .expect("orchestrator returned error");

    // --- Assertions ---

    // End signal should be VoiceCommand.
    assert_eq!(result, EndSignal::VoiceCommand);

    // Status should be Closing.
    let final_status = state.read().await.status();
    assert_eq!(final_status, Status::Closing);

    // History should contain turns from both speakers.
    let history = state.read().await.history().to_vec();
    assert!(
        history.len() >= 3,
        "expected at least 3 turns (opening + user + continuation), got {}",
        history.len()
    );

    // Verify we have both Conch and User turns.
    let has_conch = history
        .iter()
        .any(|t| matches!(t.speaker, conch::interview::history::Speaker::Conch));
    let has_user = history
        .iter()
        .any(|t| matches!(t.speaker, conch::interview::history::Speaker::User));
    assert!(has_conch, "history should contain Conch turns");
    assert!(has_user, "history should contain User turns");

    // Audio sink should have received some PCM data.
    let pcm_len = collected.lock().unwrap().len();
    assert!(pcm_len > 0, "audio sink should have received PCM data");
}

#[tokio::test]
async fn user_quit_ends_immediately() {
    let stt = Arc::new(fakes::FakeStt::new(vec![]));
    let tts = Arc::new(fakes::FakeTts);
    let llm = Arc::new(fakes::FakeLlm::new(vec![
        "Welcome! Let's chat.".to_string(),
    ]));

    let sink = fakes::FakeSink::new();
    let state = Arc::new(RwLock::new(AppState::new(
        "Quit Test".to_string(),
        "brief".to_string(),
    )));

    let (event_tx, event_rx) = mpsc::channel::<UserEvent>(16);

    let config = OrchestratorConfig {
        model: "fake-model".to_string(),
        brief: "test".to_string(),
        ..OrchestratorConfig::default()
    };

    let orch = Orchestrator::new(
        llm,
        stt,
        tts,
        Box::new(sink),
        state.clone(),
        event_rx,
        config,
    );

    let handle = tokio::spawn(async move { orch.run().await });

    // Wait for opening.
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    // Send Quit.
    event_tx.send(UserEvent::Quit).await.unwrap();

    let result = tokio::time::timeout(std::time::Duration::from_secs(5), handle)
        .await
        .expect("timed out")
        .expect("panicked")
        .expect("error");

    assert_eq!(result, EndSignal::UserQuit);
    assert_eq!(state.read().await.status(), Status::Closing);
}

#[tokio::test]
async fn llm_end_session_tool_call() {
    use conch::llm::types::{
        ChatResponse, Choice, FunctionCall, Message, Role, ToolCall,
    };

    // Custom FakeLlm that returns an end_session tool call on the second chat.
    struct EndSessionLlm {
        call_count: std::sync::Mutex<u32>,
    }

    #[async_trait::async_trait]
    impl conch::interview::orchestrator::LlmCaller for EndSessionLlm {
        async fn chat(
            &self,
            _req: &conch::llm::types::ChatRequest,
        ) -> anyhow::Result<ChatResponse> {
            let mut count = self.call_count.lock().unwrap();
            *count += 1;
            let n = *count;
            drop(count);

            if n == 1 {
                // Opening
                Ok(ChatResponse {
                    id: None,
                    choices: vec![Choice {
                        index: 0,
                        message: Message {
                            role: Role::Assistant,
                            content: Some("Hi there!".to_string()),
                            tool_calls: None,
                            tool_call_id: None,
                        },
                        finish_reason: "stop".to_string(),
                    }],
                    model: None,
                })
            } else {
                // Continuation — invoke end_session tool
                Ok(ChatResponse {
                    id: None,
                    choices: vec![Choice {
                        index: 0,
                        message: Message {
                            role: Role::Assistant,
                            content: Some("We covered everything. Thanks!".to_string()),
                            tool_calls: Some(vec![ToolCall {
                                id: "call_1".to_string(),
                                call_type: "function".to_string(),
                                function: FunctionCall {
                                    name: "end_session".to_string(),
                                    arguments: r#"{"reason":"all angles covered"}"#.to_string(),
                                },
                            }]),
                            tool_call_id: None,
                        },
                        finish_reason: "tool_calls".to_string(),
                    }],
                    model: None,
                })
            }
        }
    }

    let stt = Arc::new(fakes::FakeStt::new(vec![
        vec!["just a short answer".to_string()],
    ]));
    let tts = Arc::new(fakes::FakeTts);
    let llm = Arc::new(EndSessionLlm {
        call_count: std::sync::Mutex::new(0),
    });

    let sink = fakes::FakeSink::new();
    let state = Arc::new(RwLock::new(AppState::new(
        "Tool Test".to_string(),
        "brief".to_string(),
    )));

    let (event_tx, event_rx) = mpsc::channel::<UserEvent>(16);

    let config = OrchestratorConfig {
        model: "fake-model".to_string(),
        brief: "test".to_string(),
        ..OrchestratorConfig::default()
    };

    let orch = Orchestrator::new(
        llm,
        stt,
        tts,
        Box::new(sink),
        state.clone(),
        event_rx,
        config,
    );

    let handle = tokio::spawn(async move { orch.run().await });

    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    // One user turn
    event_tx.send(UserEvent::MicToggle).await.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    event_tx.send(UserEvent::MicToggle).await.unwrap();

    let result = tokio::time::timeout(std::time::Duration::from_secs(5), handle)
        .await
        .expect("timed out")
        .expect("panicked")
        .expect("error");

    assert_eq!(
        result,
        EndSignal::LlmEndSession {
            reason: "all angles covered".to_string()
        }
    );
    assert_eq!(state.read().await.status(), Status::Closing);
}

#[tokio::test]
async fn barge_in_interrupts_speaking_and_returns_to_idle() {
    // Use SlowTts so that the orchestrator stays in Speaking state long enough
    // for the Interrupt event to arrive. Each chunk takes 50ms, 20 chunks = 1s.
    let tts = Arc::new(fakes::SlowTts {
        chunk_count: 20,
        chunk_delay: Duration::from_millis(50),
    });

    // Two LLM replies: opening + continuation after user turn.
    let llm = Arc::new(fakes::FakeLlm::new(vec![
        "Welcome! Tell me about yourself.".to_string(),
        "That's really interesting. Can you elaborate?".to_string(),
    ]));

    // One STT script for one user turn.
    let stt = Arc::new(fakes::FakeStt::new(vec![
        vec!["I work on audio systems".to_string()],
    ]));

    let sink = fakes::FakeSink::new();
    let collected = sink.collected();

    let state = Arc::new(RwLock::new(AppState::new(
        "Barge-in Test".to_string(),
        "brief".to_string(),
    )));

    let (event_tx, event_rx) = mpsc::channel::<UserEvent>(16);

    let config = OrchestratorConfig {
        model: "fake-model".to_string(),
        brief: "test".to_string(),
        ..OrchestratorConfig::default()
    };

    let orch = Orchestrator::new(
        llm,
        stt,
        tts,
        Box::new(sink),
        state.clone(),
        event_rx,
        config,
    );

    let handle = tokio::spawn(async move { orch.run().await });

    // Wait for the opening speak to start (SlowTts takes time).
    tokio::time::sleep(Duration::from_millis(150)).await;

    // Record how many PCM samples were pushed before interrupting.
    let _before_interrupt = collected.lock().unwrap().len();

    // Barge-in during the opening speak.
    event_tx.send(UserEvent::Interrupt).await.unwrap();

    // Wait for the orchestrator to process the interrupt and go back to Idle.
    tokio::time::sleep(Duration::from_millis(100)).await;

    // The orchestrator should be back at Idle, not still Speaking.
    let status = state.read().await.status();
    assert_eq!(status, Status::Idle, "should return to Idle after barge-in");

    // Fewer PCM samples than a full 20-chunk playback would produce.
    let after_interrupt = collected.lock().unwrap().len();
    assert!(
        after_interrupt < 20 * 160,
        "barge-in should have cut playback short: got {} samples, full would be {}",
        after_interrupt,
        20 * 160
    );

    // Now do a normal user turn and quit to confirm the loop is healthy.
    event_tx.send(UserEvent::MicToggle).await.unwrap();
    tokio::time::sleep(Duration::from_millis(20)).await;
    event_tx.send(UserEvent::MicToggle).await.unwrap();

    // Wait for continuation speak to start, then quit.
    tokio::time::sleep(Duration::from_millis(150)).await;
    event_tx.send(UserEvent::Quit).await.unwrap();

    let result = tokio::time::timeout(Duration::from_secs(5), handle)
        .await
        .expect("orchestrator timed out")
        .expect("orchestrator panicked")
        .expect("orchestrator returned error");

    assert_eq!(result, EndSignal::UserQuit);
}
