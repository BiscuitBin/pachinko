"""The specification's Identity section: the two joins' triggers and tables.

Each join has one rule. When its guard does not hold, the id is taken, and the
join is refused (spec note N12).
"""

from dataclasses import dataclass
from typing import TYPE_CHECKING

from .inspection import View
from .model import Consumer, Publisher
from .refusals import ConsumerAlreadyJoined, PublisherAlreadyJoined
from .rules import SPECIFICATION, NoGuardHolds, Rule, Table

if TYPE_CHECKING:
    from .queue import Outcome


@dataclass(frozen=True)
class PublisherJoins:
    """The trigger `PublisherJoins(publisher_id)`."""

    publisher_id: str


@dataclass(frozen=True)
class ConsumerJoins:
    """The trigger `ConsumerJoins(consumer_id)`."""

    consumer_id: str


# rule PublisherJoins ------------------------------------------------------


def _publisher_joins_guard(view: View, trigger: PublisherJoins) -> bool:
    # requires: not exists Publisher{id: publisher_id}
    return not any(publisher.id == trigger.publisher_id for publisher in view.state.publishers)


def _publisher_joins(view: View, outcome: "Outcome", trigger: PublisherJoins) -> None:
    # ensures: Publisher.created(id: publisher_id)
    outcome.create(Publisher(id=trigger.publisher_id))


# rule ConsumerJoins -------------------------------------------------------


def _consumer_joins_guard(view: View, trigger: ConsumerJoins) -> bool:
    # requires: not exists Consumer{id: consumer_id}
    return not any(consumer.id == trigger.consumer_id for consumer in view.state.consumers)


def _consumer_joins(view: View, outcome: "Outcome", trigger: ConsumerJoins) -> None:
    # ensures: Consumer.created(id: consumer_id)
    outcome.create(Consumer(id=trigger.consumer_id))


PUBLISHER_JOINS = SPECIFICATION.register(
    Table(
        "PublisherJoins",
        (Rule("PublisherJoins", _publisher_joins_guard, _publisher_joins),),
        NoGuardHolds.REFUSAL,
        lambda view, trigger: PublisherAlreadyJoined(f"a publisher with id {trigger.publisher_id!r} has joined"),
    )
)

CONSUMER_JOINS = SPECIFICATION.register(
    Table(
        "ConsumerJoins",
        (Rule("ConsumerJoins", _consumer_joins_guard, _consumer_joins),),
        NoGuardHolds.REFUSAL,
        lambda view, trigger: ConsumerAlreadyJoined(f"a consumer with id {trigger.consumer_id!r} has joined"),
    )
)
