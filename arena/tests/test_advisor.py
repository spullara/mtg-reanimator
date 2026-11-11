"""Unit tests for the Claude-backed advisor. No real API calls are made."""

import sys
import unittest
from pathlib import Path
from unittest import mock

import httpx

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

import anthropic  # noqa: E402

import advisor  # noqa: E402
from advisor import AdvisorResponse, Alternative  # noqa: E402

GAME_STATE = {
    "you": {
        "hand": [{"name": "Zombify"}],
        "graveyard": [{"name": "Terror of the Peaks"}],
        "battlefield": [{"name": "Swamp"}, {"name": "Watery Grave"}],
        "exile": [],
    },
    "opponent": {"battlefield": []},
}

ACTIONS = [
    {"name": "Cast Zombify", "actionType": "ActionType_Cast", "actionIndex": 0},
    {"name": "Pass", "actionType": "ActionType_Pass", "actionIndex": 1},
]


def _parsed_response(stop_reason="end_turn", parsed=None):
    response = mock.Mock()
    response.stop_reason = stop_reason
    response.parsed_output = parsed
    return response


def _good_parsed():
    return AdvisorResponse(
        recommended_action=0,
        reasoning="Zombify returns Terror of the Peaks.",
        alternatives=[Alternative(action=1, reasoning="Pass and hold up nothing.")],
    )


class AdvisorRequestTests(unittest.TestCase):
    def _call_and_capture(self):
        client = mock.Mock()
        client.beta.messages.parse.return_value = _parsed_response(parsed=_good_parsed())
        clean_env = {"ARENA_ADVISOR_MODEL": "", "ARENA_ADVISOR_EFFORT": ""}
        with mock.patch.object(advisor, "_sync_client", return_value=client), \
                mock.patch.dict("os.environ", clean_env):
            for key in clean_env:
                import os
                os.environ.pop(key, None)
            result = advisor.advise_move(GAME_STATE, ACTIONS)
        self.assertEqual(client.beta.messages.parse.call_count, 1)
        return result, client.beta.messages.parse.call_args.kwargs

    def test_good_response_is_used(self):
        result, _ = self._call_and_capture()
        self.assertEqual(result["recommended_action"], 0)
        self.assertEqual(result["reasoning"], "Zombify returns Terror of the Peaks.")
        self.assertEqual(result["alternatives"], [{"action": 1, "reasoning": "Pass and hold up nothing."}])

    def test_request_shape(self):
        _, kwargs = self._call_and_capture()
        self.assertNotIn("temperature", kwargs)
        self.assertNotIn("top_p", kwargs)
        self.assertNotIn("top_k", kwargs)
        self.assertNotIn("thinking", kwargs)
        self.assertEqual(kwargs["model"], "claude-opus-5-5")
        self.assertEqual(kwargs["max_tokens"], 16000)
        self.assertEqual(kwargs["output_config"], {"effort": "low"})
        self.assertEqual(kwargs["output_format"], AdvisorResponse)
        self.assertEqual(kwargs["betas"], ["server-side-fallback-2026-07-01"])
        self.assertEqual(kwargs["fallbacks"], "default")
        system = kwargs["system"]
        self.assertEqual(system[0]["cache_control"], {"type": "ephemeral"})
        self.assertIn("Dimir Reanimator", system[0]["text"])
        self.assertNotIn("Return only valid JSON", system[0]["text"])

    def test_env_overrides(self):
        env = {"ARENA_ADVISOR_MODEL": "claude-test-model", "ARENA_ADVISOR_EFFORT": "high"}
        with mock.patch.dict("os.environ", env):
            kwargs = advisor._request_kwargs("prompt")
        self.assertEqual(kwargs["model"], "claude-test-model")
        self.assertEqual(kwargs["output_config"], {"effort": "high"})


class AdvisorFallbackTests(unittest.TestCase):
    def _advise_with(self, client):
        with mock.patch.object(advisor, "_sync_client", return_value=client):
            return advisor.advise_move(GAME_STATE, ACTIONS)

    def test_refusal_falls_back_to_heuristic(self):
        client = mock.Mock()
        client.beta.messages.parse.return_value = _parsed_response(stop_reason="refusal")
        result = self._advise_with(client)
        self.assertIn("LLM unavailable; offline heuristic", result["reasoning"])
        self.assertIn("refusal", result["reasoning"])
        self.assertIn(result["recommended_action"], {0, 1})

    def test_connection_error_falls_back_to_heuristic(self):
        client = mock.Mock()
        client.beta.messages.parse.side_effect = anthropic.APIConnectionError(
            request=httpx.Request("POST", "https://api.anthropic.com/v1/messages")
        )
        result = self._advise_with(client)
        self.assertIn("LLM unavailable; offline heuristic", result["reasoning"])
        self.assertIn("connection", result["reasoning"].lower())

    def test_offline_mode_never_builds_a_client(self):
        with mock.patch.object(advisor, "_sync_client") as client_factory:
            result = advisor.advise_move(GAME_STATE, ACTIONS, use_llm=False)
        client_factory.assert_not_called()
        self.assertIn(result["recommended_action"], {0, 1})


class AdvisorAsyncTests(unittest.IsolatedAsyncioTestCase):
    async def test_async_uses_async_client(self):
        client = mock.Mock()
        client.beta.messages.parse = mock.AsyncMock(
            return_value=_parsed_response(parsed=_good_parsed())
        )
        with mock.patch.object(advisor, "_async_client", return_value=client) as factory, \
                mock.patch.object(advisor, "_sync_client") as sync_factory:
            result = await advisor.get_advice_async(GAME_STATE, ACTIONS)
        factory.assert_called_once()
        sync_factory.assert_not_called()
        self.assertEqual(result["recommended_action"], 0)


if __name__ == "__main__":
    unittest.main()
