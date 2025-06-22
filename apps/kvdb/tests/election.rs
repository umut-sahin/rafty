//! Election tests.

use {
    rafty::prelude::*,
    rafty_kvdb::*,
    rafty_simulator::*,
};

mod storage;
use storage::Storage;

#[test]
fn single_candidate_election() -> anyhow::Result<()> {
    env_logger::init();

    let consistency = Consistency::Strong;

    let number_of_clients = 2;
    let number_of_peers = 5;

    let initial_peer_storages = vec![Storage::default(); number_of_peers];
    let replay_storages = initial_peer_storages.clone();

    let no_op_entry = LogEntry::builder().index(1).term(1).command(Command::NoOp).build();
    let initial_append_entries_request = AppendEntriesRequest::builder()
        .term(1)
        .leader_id(2)
        .prev_log_index(0)
        .prev_log_term(0)
        .entries([no_op_entry.clone()])
        .leader_commit(0)
        .build();
    let heartbeat_request = AppendEntriesRequest::builder()
        .term(1)
        .leader_id(2)
        .prev_log_index(1)
        .prev_log_term(1)
        .entries([])
        .leader_commit(1)
        .build();

    Simulation::<KeyValueDatabase<Storage>>::new(
        consistency,
        initial_peer_storages,
        number_of_clients,
    )?
    .enable_checks(replay_storages)?
    .run(
        [
            // Cluster initializes
            Action::Check {
                updates: (1..=number_of_peers)
                    .map(|peer_id| {
                        Update::peer(peer_id)
                            .set_term(0)
                            .set_voted_for(None)
                            .set_log(vec![])
                            .set_commit_index(0)
                            .set_last_applied(0)
                            .set_role(Role::Follower(
                                FollowerState::builder().leader_id(None).build(),
                            ))
                            .set_machine(Machine::default())
                            .set_snapshot(Snapshot::default())
                            .clear_buffered_peer_transmits()
                    })
                    .collect(),
            }, // #0
            //-----------------------------
            // Election times out on Peer 2
            Action::TimeoutElection { peer_id: PeerId(2) }, // #1
            Action::Check {
                updates: vec![Update::peer(2)
                    .set_term(1)
                    .set_voted_for(Some(PeerId(2)))
                    .set_role(Role::Candidate(
                        CandidateState::builder()
                            .votes_granted(1)
                            .vote_request_ids([0, 1, 2, 3].into_iter().map(RequestId))
                            .build(),
                    ))
                    .set_buffered_peer_transmits(
                        (1..=number_of_peers).filter(|peer_id| *peer_id != 2).enumerate().map(
                            |(request_id, peer_id)| {
                                let request = RequestVoteRequest::builder()
                                    .term(1)
                                    .candidate_id(2)
                                    .last_log_index(0)
                                    .last_log_term(0)
                                    .build();
                                PeerTransmit::builder()
                                    .peer_id(peer_id)
                                    .request_id(request_id)
                                    .message(request)
                                    .build()
                            },
                        ),
                    )],
            }, // #2
            //-----------------------------
            // Peer 2 sends `RequestVote` requests to other Peers
            Action::TransmitPeerRequests {
                peer_id: PeerId(2),
                request_ids: [0, 1, 2, 3].into_iter().map(RequestId).collect(),
            }, // #3
            Action::Check {
                updates: (1..=number_of_peers)
                    .filter(|peer_id| *peer_id != 2)
                    .enumerate()
                    .map(|(request_id, peer_id)| {
                        Update::peer(peer_id)
                            .set_term(1)
                            .set_voted_for(Some(PeerId(2)))
                            .set_buffered_peer_transmits(std::iter::once(
                                PeerTransmit::builder()
                                    .peer_id(2)
                                    .request_id(request_id)
                                    .message(
                                        RequestVoteReply::builder()
                                            .term(1)
                                            .vote(Vote::Granted)
                                            .build(),
                                    )
                                    .build(),
                            ))
                    })
                    .chain(std::iter::once(Update::peer(2).clear_buffered_peer_transmits()))
                    .collect(),
            }, // #4
            //-----------------------------
            // Peer 1 replies to `RequestVote` request #0 of Peer 2
            Action::TransmitPeerReply {
                peer_id: PeerId(1),
                replied_peer_id_and_request_id: (PeerId(2), RequestId(0)),
            }, // #5
            Action::Check {
                #[rustfmt::skip]
                    updates: std::iter::once(
                        Update::peer(1).clear_buffered_peer_transmits(),
                    )
                    .chain(std::iter::once(
                        Update::peer(2)
                            .set_role(
                                Role::Candidate(
                                    CandidateState::builder()
                                        .votes_granted(2)
                                        .vote_request_ids([1, 2, 3].into_iter().map(RequestId))
                                        .build()
                                )
                            )
                    ))
                    .collect(),
            }, // #6
            //-----------------------------
            // Peer 3 replies to `RequestVote` request #1 of Peer 2
            Action::TransmitPeerReply {
                peer_id: PeerId(3),
                replied_peer_id_and_request_id: (PeerId(2), RequestId(1)),
            }, // #7
            Action::Check {
                #[rustfmt::skip]
                    updates: std::iter::once(
                        Update::peer(3).clear_buffered_peer_transmits(),
                    )
                    .chain(std::iter::once(
                        Update::peer(2)
                            .set_log(vec![no_op_entry.clone()])
                            .set_role(
                                Role::Leader(
                                    LeaderState::builder()
                                        .next_index(
                                            [
                                                (PeerId(1), LogIndex(2)),
                                                (PeerId(3), LogIndex(2)),
                                                (PeerId(4), LogIndex(2)),
                                                (PeerId(5), LogIndex(2)),
                                            ]
                                        )
                                        .match_index(
                                            [
                                                (PeerId(1), LogIndex(0)),
                                                (PeerId(2), LogIndex(1)),
                                                (PeerId(3), LogIndex(0)),
                                                (PeerId(4), LogIndex(0)),
                                                (PeerId(5), LogIndex(0)),
                                            ]
                                        )
                                        .append_entries_requests(
                                            (1..=number_of_peers)
                                                .filter(|peer_id| *peer_id != 2)
                                                .enumerate()
                                                .map(|(mut request_id, _)| {
                                                    request_id += 4;
                                                    (
                                                        request_id.into(),
                                                        initial_append_entries_request.clone(),
                                                    )
                                                })
                                        )
                                        .build()
                                )
                            )
                            .set_buffered_peer_transmits(
                                (1..=number_of_peers)
                                    .filter(|peer_id| *peer_id != 2)
                                    .enumerate()
                                    .map(|(mut request_id, peer_id)| {
                                        request_id += 4;
                                        PeerTransmit::builder()
                                            .peer_id(peer_id)
                                            .request_id(request_id)
                                            .message(initial_append_entries_request.clone())
                                            .build()
                                    }),
                            ),
                    ))
                    .collect(),
            }, // #8
            //-----------------------------
            // Peer 4 replies to `RequestVote` request #2 of Peer 2
            Action::TransmitPeerReply {
                peer_id: PeerId(4),
                replied_peer_id_and_request_id: (PeerId(2), RequestId(2)),
            }, // #9
            Action::Check {
                updates: std::iter::once(Update::peer(4).clear_buffered_peer_transmits()).collect(),
            }, // #10
            //-----------------------------
            // Peer 5 replies to `RequestVote` request #3 of Peer 2
            Action::TransmitPeerReply {
                peer_id: PeerId(5),
                replied_peer_id_and_request_id: (PeerId(2), RequestId(3)),
            }, // #11
            Action::Check {
                updates: std::iter::once(Update::peer(5).clear_buffered_peer_transmits()).collect(),
            }, // #12
            //-----------------------------
            // Peer 2 sends the initial `AppendEntries` requests to other Peers
            Action::TransmitPeerRequests {
                peer_id: PeerId(2),
                request_ids: [4, 5, 6, 7].into_iter().map(RequestId).collect(),
            }, // #13
            Action::Check {
                updates: (1..=number_of_peers)
                    .filter(|peer_id| *peer_id != 2)
                    .enumerate()
                    .map(|(mut request_id, peer_id)| {
                        request_id += 4;
                        Update::peer(peer_id)
                            .set_log(vec![no_op_entry.clone()])
                            .set_role(Role::Follower(
                                FollowerState::builder().leader_id(PeerId(2)).build(),
                            ))
                            .set_buffered_peer_transmits(std::iter::once(
                                PeerTransmit::builder()
                                    .peer_id(2)
                                    .request_id(request_id)
                                    .message(
                                        AppendEntriesReply::builder().term(1).success(true).build(),
                                    )
                                    .build(),
                            ))
                    })
                    .chain(std::iter::once(Update::peer(2).clear_buffered_peer_transmits()))
                    .collect(),
            }, // #14
            //-----------------------------
            // Peer 1 replies to `AppendEntries` request #4 of Peer 2
            Action::TransmitPeerReply {
                peer_id: PeerId(1),
                replied_peer_id_and_request_id: (PeerId(2), RequestId(4)),
            }, // #15
            Action::Check {
                #[rustfmt::skip]
                    updates: std::iter::once(
                        Update::peer(1).clear_buffered_peer_transmits(),
                    )
                    .chain(std::iter::once(
                        Update::peer(2)
                            .set_role(
                                Role::Leader(
                                    LeaderState::builder()
                                        .next_index(
                                            [
                                                (PeerId(1), LogIndex(2)),
                                                (PeerId(3), LogIndex(2)),
                                                (PeerId(4), LogIndex(2)),
                                                (PeerId(5), LogIndex(2)),
                                            ]
                                        )
                                        .match_index(
                                            [
                                                (PeerId(1), LogIndex(1)),
                                                (PeerId(2), LogIndex(1)),
                                                (PeerId(3), LogIndex(0)),
                                                (PeerId(4), LogIndex(0)),
                                                (PeerId(5), LogIndex(0)),
                                            ]
                                        )
                                        .append_entries_requests(
                                            (1..=number_of_peers)
                                                .filter(|peer_id| *peer_id != 2)
                                                .enumerate()
                                                .filter_map(|(mut request_id, _)| {
                                                    request_id += 4;
                                                    match request_id {
                                                        4 => None,
                                                        _ => Some(
                                                            (
                                                                request_id.into(),
                                                                initial_append_entries_request
                                                                    .clone()
                                                            )
                                                        ),
                                                    }
                                                })
                                        )
                                        .build()
                                )
                            )
                    ))
                    .collect(),
            }, // #16
            //-----------------------------
            // Peer 3 replies to `AppendEntries` request #5 of Peer 2
            Action::TransmitPeerReply {
                peer_id: PeerId(3),
                replied_peer_id_and_request_id: (PeerId(2), RequestId(5)),
            }, // #17
            Action::Check {
                #[rustfmt::skip]
                    updates: std::iter::once(
                        Update::peer(3).clear_buffered_peer_transmits(),
                    )
                    .chain(std::iter::once(
                        Update::peer(2)
                            .set_commit_index(1)
                            .set_role(
                                Role::Leader(
                                    LeaderState::builder()
                                        .next_index(
                                            [
                                                (PeerId(1), LogIndex(2)),
                                                (PeerId(3), LogIndex(2)),
                                                (PeerId(4), LogIndex(2)),
                                                (PeerId(5), LogIndex(2)),
                                            ]
                                        )
                                        .match_index(
                                            [
                                                (PeerId(1), LogIndex(1)),
                                                (PeerId(2), LogIndex(1)),
                                                (PeerId(3), LogIndex(1)),
                                                (PeerId(4), LogIndex(0)),
                                                (PeerId(5), LogIndex(0)),
                                            ]
                                        )
                                        .append_entries_requests(
                                            (1..=number_of_peers)
                                                .filter(|peer_id| *peer_id != 2)
                                                .enumerate()
                                                .filter_map(|(mut request_id, _)| {
                                                    request_id += 4;
                                                    match request_id {
                                                        4 | 5 => None,
                                                        _ => Some(
                                                            (
                                                                request_id.into(),
                                                                initial_append_entries_request
                                                                    .clone()
                                                            )
                                                        ),
                                                    }
                                                })
                                        )
                                        .build()
                                )
                            )
                    ))
                    .collect(),
            }, // #18
            //-----------------------------
            // Peers apply committed entries
            Action::ApplyCommitted { peer_id: None }, // #18
            Action::Check { updates: vec![Update::peer(2).set_last_applied(1)] }, // #19
            //-----------------------------
            // Peer 4 replies to `AppendEntries` request #6 of Peer 2
            Action::TransmitPeerReply {
                peer_id: PeerId(4),
                replied_peer_id_and_request_id: (PeerId(2), RequestId(6)),
            }, // #20
            Action::Check {
                #[rustfmt::skip]
                    updates: std::iter::once(
                        Update::peer(4).clear_buffered_peer_transmits(),
                    )
                    .chain(std::iter::once(
                        Update::peer(2)
                            .set_role(
                                Role::Leader(
                                    LeaderState::builder()
                                        .next_index(
                                            [
                                                (PeerId(1), LogIndex(2)),
                                                (PeerId(3), LogIndex(2)),
                                                (PeerId(4), LogIndex(2)),
                                                (PeerId(5), LogIndex(2)),
                                            ]
                                        )
                                        .match_index(
                                            [
                                                (PeerId(1), LogIndex(1)),
                                                (PeerId(2), LogIndex(1)),
                                                (PeerId(3), LogIndex(1)),
                                                (PeerId(4), LogIndex(1)),
                                                (PeerId(5), LogIndex(0)),
                                            ]
                                        )
                                        .append_entries_requests(
                                            (1..=number_of_peers)
                                                .filter(|peer_id| *peer_id != 2)
                                                .enumerate()
                                                .filter_map(|(mut request_id, _)| {
                                                    request_id += 4;
                                                    match request_id {
                                                        4..=6 => None,
                                                        _ => Some(
                                                            (
                                                                request_id.into(),
                                                                initial_append_entries_request
                                                                    .clone()
                                                            )
                                                        ),
                                                    }
                                                })
                                        )
                                        .build()
                                )
                            )
                    ))
                    .collect(),
            }, // #21
            //-----------------------------
            // Peer 5 replies to `AppendEntries` request #7 of Peer 2
            Action::TransmitPeerReply {
                peer_id: PeerId(5),
                replied_peer_id_and_request_id: (PeerId(2), RequestId(7)),
            }, // #22
            Action::Check {
                #[rustfmt::skip]
                    updates: std::iter::once(
                        Update::peer(5).clear_buffered_peer_transmits(),
                    )
                    .chain(std::iter::once(
                        Update::peer(2)
                            .set_role(
                                Role::Leader(
                                    LeaderState::builder()
                                        .next_index(
                                            [
                                                (PeerId(1), LogIndex(2)),
                                                (PeerId(3), LogIndex(2)),
                                                (PeerId(4), LogIndex(2)),
                                                (PeerId(5), LogIndex(2)),
                                            ]
                                        )
                                        .match_index(
                                            [
                                                (PeerId(1), LogIndex(1)),
                                                (PeerId(2), LogIndex(1)),
                                                (PeerId(3), LogIndex(1)),
                                                (PeerId(4), LogIndex(1)),
                                                (PeerId(5), LogIndex(1)),
                                            ]
                                        )
                                        .build()
                                )
                            )
                    ))
                    .collect(),
            }, // #23
            //-----------------------------
            // Heartbeat times out on Peer 2
            Action::TimeoutHeartbeat { peer_id: PeerId(2) }, // #24
            Action::Check {
                #[rustfmt::skip]
                    updates: vec![
                        Update::peer(2)
                            .set_role(
                                Role::Leader(
                                    LeaderState::builder()
                                        .next_index(
                                            [
                                                (PeerId(1), LogIndex(2)),
                                                (PeerId(3), LogIndex(2)),
                                                (PeerId(4), LogIndex(2)),
                                                (PeerId(5), LogIndex(2)),
                                            ]
                                        )
                                        .match_index(
                                            [
                                                (PeerId(1), LogIndex(1)),
                                                (PeerId(2), LogIndex(1)),
                                                (PeerId(3), LogIndex(1)),
                                                (PeerId(4), LogIndex(1)),
                                                (PeerId(5), LogIndex(1)),
                                            ]
                                        )
                                        .append_entries_requests(
                                            (1..=number_of_peers)
                                                .filter(|peer_id| *peer_id != 2)
                                                .enumerate()
                                                .map(|(mut request_id, _)| {
                                                    request_id += 8;
                                                    (
                                                        request_id.into(),
                                                        heartbeat_request.clone()
                                                    )
                                                })
                                        )
                                        .build()
                                )
                            )
                            .set_buffered_peer_transmits(
                                (1..=number_of_peers)
                                    .filter(|peer_id| *peer_id != 2)
                                    .enumerate()
                                    .map(|(mut request_id, peer_id)| {
                                        request_id += 8;
                                        PeerTransmit::builder()
                                            .peer_id(peer_id)
                                            .request_id(request_id)
                                            .message(heartbeat_request.clone())
                                            .build()
                                    }),
                            ),
                    ],
            }, // #25
            //-----------------------------
            // Peer 2 sends `AppendEntries` requests to other Peers
            Action::TransmitPeerRequests {
                peer_id: PeerId(2),
                request_ids: [8, 9, 10, 11].into_iter().map(RequestId).collect(),
            }, // #26
            Action::Check {
                updates: (1..=number_of_peers)
                    .filter(|peer_id| *peer_id != 2)
                    .enumerate()
                    .map(|(mut request_id, peer_id)| {
                        request_id += 8;
                        Update::peer(peer_id)
                            .set_commit_index(1)
                            .set_last_applied(0)
                            .set_buffered_peer_transmits(std::iter::once(
                                PeerTransmit::builder()
                                    .peer_id(2)
                                    .request_id(request_id)
                                    .message(
                                        AppendEntriesReply::builder().term(1).success(true).build(),
                                    )
                                    .build(),
                            ))
                    })
                    .chain(std::iter::once(Update::peer(2).clear_buffered_peer_transmits()))
                    .collect(),
            }, // #27
            //-----------------------------
            // Peers apply committed entries
            Action::ApplyCommitted { peer_id: None }, // #28
            Action::Check {
                updates: (1..=number_of_peers)
                    .filter(|peer_id| *peer_id != 2)
                    .map(|peer_id| Update::peer(peer_id).set_last_applied(1))
                    .collect(),
            }, // #29
        ]
        .into_iter(),
    )
}
